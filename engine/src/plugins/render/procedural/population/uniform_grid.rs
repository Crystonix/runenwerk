use crate::plugins::render::gpu_primitives::{
    CounterResetDescriptor, GpuPrimitiveExecutionPlan, GpuPrimitiveStep,
    GpuPrimitiveValidationError, PrefixScanMode, U32Counter, U32PrefixScanDescriptor,
    U32ScanElement, buffer_capacity, validate_capacity,
};
use runen_gpu::{GpuBufferHandle, GpuWorkResourceId};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedUniformGrid2dConfig {
    pub cells_x: u32,
    pub cells_y: u32,
    pub max_agents: u32,
}

impl BoundedUniformGrid2dConfig {
    pub const fn new(cells_x: u32, cells_y: u32, max_agents: u32) -> Self {
        Self {
            cells_x,
            cells_y,
            max_agents,
        }
    }

    pub fn cell_count(self) -> u32 {
        self.cells_x
            .checked_mul(self.cells_y)
            .expect("bounded uniform grid config must be validated before cell_count")
    }

    pub fn checked_cell_count(self) -> Result<u32, PopulationGridValidationError> {
        self.cells_x.checked_mul(self.cells_y).ok_or(
            PopulationGridValidationError::GridCellCountOverflow {
                cells_x: self.cells_x,
                cells_y: self.cells_y,
            },
        )
    }

    pub fn validate(self) -> Result<(), PopulationGridValidationError> {
        if self.cells_x == 0 || self.cells_y == 0 {
            return Err(PopulationGridValidationError::InvalidGridDimensions {
                cells_x: self.cells_x,
                cells_y: self.cells_y,
            });
        }
        if self.max_agents == 0 {
            return Err(PopulationGridValidationError::ZeroAgentCapacity);
        }
        self.checked_cell_count()?;
        Ok(())
    }

    pub fn wrapped_cell_index(
        self,
        cell_x: i64,
        cell_y: i64,
    ) -> Result<u32, PopulationGridValidationError> {
        self.validate()?;
        let wrapped_x = wrap_coordinate(cell_x, self.cells_x);
        let wrapped_y = wrap_coordinate(cell_y, self.cells_y);
        Ok(wrapped_y * self.cells_x + wrapped_x)
    }

    pub fn adjacent_cell_indices(
        self,
        center_x: u32,
        center_y: u32,
    ) -> Result<[u32; 9], PopulationGridValidationError> {
        self.validate()?;
        if center_x >= self.cells_x || center_y >= self.cells_y {
            return Err(PopulationGridValidationError::CellOutOfRange {
                cell_x: center_x,
                cell_y: center_y,
                cells_x: self.cells_x,
                cells_y: self.cells_y,
            });
        }

        let mut indices = [0; 9];
        let mut out = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                indices[out] =
                    self.wrapped_cell_index(i64::from(center_x) + dx, i64::from(center_y) + dy)?;
                out += 1;
            }
        }
        Ok(indices)
    }
}

fn wrap_coordinate(value: i64, dimension: u32) -> u32 {
    value.rem_euclid(i64::from(dimension)) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundedUniformGrid2dStage {
    ClearCounts,
    CountCells,
    ScanCounts,
    ResetCursors,
    ScatterSortedIndices,
    SimulateNeighbors,
    PublishDraw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedUniformGrid2dStagePlan {
    pub stage: BoundedUniformGrid2dStage,
    pub label: String,
}

impl BoundedUniformGrid2dStagePlan {
    pub fn new(stage: BoundedUniformGrid2dStage, label: impl Into<String>) -> Self {
        Self {
            stage,
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedUniformGrid2dResources {
    pub cell_counts: GpuWorkResourceId,
    pub cell_offsets: GpuWorkResourceId,
    pub scatter_cursors: GpuWorkResourceId,
    pub sorted_indices: GpuWorkResourceId,
    pub cell_capacity: u32,
    pub sorted_index_capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedUniformGrid2dBuildPlan {
    pub label: String,
    pub config: BoundedUniformGrid2dConfig,
    pub resources: BoundedUniformGrid2dResources,
    pub reset_counts: CounterResetDescriptor,
    pub scan_counts: U32PrefixScanDescriptor,
    pub reset_cursors: CounterResetDescriptor,
    pub primitive_plan: GpuPrimitiveExecutionPlan,
    pub stages: Vec<BoundedUniformGrid2dStagePlan>,
}

impl BoundedUniformGrid2dBuildPlan {
    pub fn new(
        label: impl Into<String>,
        config: BoundedUniformGrid2dConfig,
        cell_counts: GpuBufferHandle,
        cell_offsets: GpuBufferHandle,
        scatter_cursors: GpuBufferHandle,
        sorted_indices: GpuBufferHandle,
    ) -> Result<Self, PopulationGridValidationError> {
        config.validate()?;
        let label = label.into();
        if label.trim().is_empty() {
            return Err(PopulationGridValidationError::EmptyLabel);
        }

        let cell_count = config.cell_count();
        let reset_counts = CounterResetDescriptor::new(
            format!("{label}.clear_counts"),
            cell_counts.clone(),
            cell_count,
        )?;
        let scan_counts = U32PrefixScanDescriptor::new(
            format!("{label}.scan_counts"),
            cell_counts.clone(),
            cell_offsets.clone(),
            cell_count,
            PrefixScanMode::Exclusive,
        )
        .map_err(PopulationGridValidationError::Primitive)?;
        let reset_cursors = CounterResetDescriptor::new(
            format!("{label}.reset_cursors"),
            scatter_cursors.clone(),
            cell_count,
        )?;

        validate_capacity(
            format!("{label}.cell_counts"),
            buffer_capacity(
                &cell_counts,
                U32Counter::BYTE_SIZE,
                format!("{label}.cell_counts"),
            )?,
            u64::from(cell_count),
        )?;
        validate_capacity(
            format!("{label}.cell_offsets"),
            buffer_capacity(
                &cell_offsets,
                U32ScanElement::BYTE_SIZE,
                format!("{label}.cell_offsets"),
            )?,
            u64::from(cell_count),
        )?;
        validate_capacity(
            format!("{label}.scatter_cursors"),
            buffer_capacity(
                &scatter_cursors,
                U32Counter::BYTE_SIZE,
                format!("{label}.scatter_cursors"),
            )?,
            u64::from(cell_count),
        )?;
        validate_capacity(
            format!("{label}.sorted_indices"),
            buffer_capacity(
                &sorted_indices,
                U32ScanElement::BYTE_SIZE,
                format!("{label}.sorted_indices"),
            )?,
            u64::from(config.max_agents),
        )?;

        let primitive_plan = GpuPrimitiveExecutionPlan::new(
            format!("{label}.primitive_plan"),
            [
                GpuPrimitiveStep::from(reset_counts.clone()),
                GpuPrimitiveStep::from(scan_counts.clone()),
                GpuPrimitiveStep::from(reset_cursors.clone()),
            ],
        )?;
        let stages = canonical_stage_plan(&label);

        Ok(Self {
            label,
            config,
            resources: BoundedUniformGrid2dResources {
                cell_counts: cell_counts.diagnostic_identity(),
                cell_offsets: cell_offsets.diagnostic_identity(),
                scatter_cursors: scatter_cursors.diagnostic_identity(),
                sorted_indices: sorted_indices.diagnostic_identity(),
                cell_capacity: cell_count,
                sorted_index_capacity: config.max_agents,
            },
            reset_counts,
            scan_counts,
            reset_cursors,
            primitive_plan,
            stages,
        })
    }
}

fn canonical_stage_plan(label: &str) -> Vec<BoundedUniformGrid2dStagePlan> {
    let stages = [
        (
            BoundedUniformGrid2dStage::ClearCounts,
            format!("{label}.clear_counts"),
        ),
        (
            BoundedUniformGrid2dStage::CountCells,
            format!("{label}.count_cells"),
        ),
        (
            BoundedUniformGrid2dStage::ScanCounts,
            format!("{label}.scan_counts"),
        ),
        (
            BoundedUniformGrid2dStage::ResetCursors,
            format!("{label}.reset_cursors"),
        ),
        (
            BoundedUniformGrid2dStage::ScatterSortedIndices,
            format!("{label}.scatter_sorted_indices"),
        ),
        (
            BoundedUniformGrid2dStage::SimulateNeighbors,
            format!("{label}.simulate_neighbors"),
        ),
        (
            BoundedUniformGrid2dStage::PublishDraw,
            format!("{label}.publish_draw"),
        ),
    ];

    stages
        .into_iter()
        .map(|(stage, label)| BoundedUniformGrid2dStagePlan::new(stage, label))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PopulationGridValidationError {
    #[error("bounded uniform grid label must not be empty")]
    EmptyLabel,

    #[error("bounded uniform grid dimensions must be non-zero; got {cells_x}x{cells_y}")]
    InvalidGridDimensions { cells_x: u32, cells_y: u32 },

    #[error("bounded uniform grid max agent capacity must be greater than zero")]
    ZeroAgentCapacity,

    #[error("bounded uniform grid cell count overflow for dimensions {cells_x}x{cells_y}")]
    GridCellCountOverflow { cells_x: u32, cells_y: u32 },

    #[error(
        "bounded uniform grid cell {cell_x},{cell_y} is outside dimensions {cells_x}x{cells_y}"
    )]
    CellOutOfRange {
        cell_x: u32,
        cell_y: u32,
        cells_x: u32,
        cells_y: u32,
    },

    #[error("bounded uniform grid '{label}' scan requires distinct count and offset buffers")]
    ScanRequiresDistinctBuffers { label: String },

    #[error(transparent)]
    Primitive(#[from] GpuPrimitiveValidationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::{RenderFlow, U32Counter};

    #[test]
    fn bounded_uniform_grid_rejects_zero_dimensions() {
        assert!(matches!(
            BoundedUniformGrid2dConfig::new(0, 16, 1024).validate(),
            Err(PopulationGridValidationError::InvalidGridDimensions { .. })
        ));
    }

    #[test]
    fn bounded_uniform_grid_rejects_zero_agent_capacity() {
        assert!(matches!(
            BoundedUniformGrid2dConfig::new(16, 16, 0).validate(),
            Err(PopulationGridValidationError::ZeroAgentCapacity)
        ));
    }

    #[test]
    fn bounded_uniform_grid_rejects_cell_count_overflow() {
        assert!(matches!(
            BoundedUniformGrid2dConfig::new(u32::MAX, 2, 1024).validate(),
            Err(PopulationGridValidationError::GridCellCountOverflow { .. })
        ));
    }

    #[test]
    fn bounded_uniform_grid_wraps_adjacent_cell_indices() {
        let config = BoundedUniformGrid2dConfig::new(4, 3, 64);

        assert_eq!(config.wrapped_cell_index(-1, -1).unwrap(), 11);
        assert_eq!(
            config.adjacent_cell_indices(0, 0).unwrap(),
            [11, 8, 9, 3, 0, 1, 7, 4, 5]
        );
    }

    #[test]
    fn bounded_uniform_grid_plan_is_total_count_sized() {
        let (flow, counts) = RenderFlow::new("test.population.grid")
            .storage_array::<U32Counter>("grid.counts", 256)
            .expect("render flow authoring should succeed");
        let (flow, offsets) = flow
            .storage_array::<U32ScanElement>("grid.offsets", 256)
            .expect("render flow authoring should succeed");
        let (flow, cursors) = flow
            .storage_array::<U32Counter>("grid.cursors", 256)
            .expect("render flow authoring should succeed");
        let (_flow, sorted) = flow
            .storage_array::<U32ScanElement>("grid.sorted", 1024)
            .expect("render flow authoring should succeed");

        let plan = BoundedUniformGrid2dBuildPlan::new(
            "grid",
            BoundedUniformGrid2dConfig::new(16, 16, 1024),
            counts,
            offsets,
            cursors,
            sorted,
        )
        .expect("valid grid plan should be created");

        assert_eq!(plan.config.cell_count(), 256);
        assert_eq!(plan.resources.cell_capacity, 256);
        assert_eq!(plan.resources.sorted_index_capacity, 1024);
        assert_eq!(plan.primitive_plan.step_count(), 3);
        assert_eq!(
            plan.stages
                .iter()
                .map(|stage| stage.stage)
                .collect::<Vec<_>>(),
            vec![
                BoundedUniformGrid2dStage::ClearCounts,
                BoundedUniformGrid2dStage::CountCells,
                BoundedUniformGrid2dStage::ScanCounts,
                BoundedUniformGrid2dStage::ResetCursors,
                BoundedUniformGrid2dStage::ScatterSortedIndices,
                BoundedUniformGrid2dStage::SimulateNeighbors,
                BoundedUniformGrid2dStage::PublishDraw,
            ]
        );
        assert_eq!(plan.stages[0].label, "grid.clear_counts");
        assert_eq!(plan.stages[1].label, "grid.count_cells");
    }
}
