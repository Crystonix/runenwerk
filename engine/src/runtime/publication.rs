use anyhow::Result;
use ecs::{Runtime, ScheduleBoundary, ScheduleLabel, World};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PublicationBoundary {
    pub index: usize,
    pub schedule_label: &'static str,
    pub stage_index: usize,
}

impl PublicationBoundary {
    pub const fn new(index: usize, schedule_label: &'static str, stage_index: usize) -> Self {
        Self {
            index,
            schedule_label,
            stage_index,
        }
    }

    fn from_ecs(index: usize, boundary: ScheduleBoundary) -> Self {
        Self::new(index, boundary.schedule().name(), boundary.stage_index())
    }
}

type PublicationHandler = Box<dyn Fn(&PublicationBoundary, &mut World) -> Result<()>>;

#[derive(Default)]
pub(crate) struct PublicationHandlers {
    product: Vec<PublicationHandler>,
    query_snapshot: Vec<PublicationHandler>,
    next_boundary_index: usize,
}

impl ecs::Resource for PublicationHandlers {}

impl PublicationHandlers {
    pub(crate) fn add_product<F>(&mut self, handler: F)
    where
        F: Fn(&PublicationBoundary, &mut World) -> Result<()> + 'static,
    {
        self.product.push(Box::new(handler));
    }

    pub(crate) fn add_query_snapshot<F>(&mut self, handler: F)
    where
        F: Fn(&PublicationBoundary, &mut World) -> Result<()> + 'static,
    {
        self.query_snapshot.push(Box::new(handler));
    }

    fn dispatch(&mut self, ecs_boundary: ScheduleBoundary, world: &mut World) -> Result<()> {
        let boundary = PublicationBoundary::from_ecs(self.next_boundary_index, ecs_boundary);
        self.next_boundary_index = self.next_boundary_index.saturating_add(1);

        for handler in &self.product {
            handler(&boundary, world)?;
        }
        for handler in &self.query_snapshot {
            handler(&boundary, world)?;
        }
        Ok(())
    }
}

pub(crate) fn run_schedule_with_publication<L: ScheduleLabel>(
    world: &mut World,
    runtime: &mut Runtime,
) -> Result<()> {
    let mut publications = world
        .remove_resource::<PublicationHandlers>()
        .unwrap_or_default();
    let result = runtime.run_schedule_with_boundary::<L, _>(world, |boundary, world| {
        publications.dispatch(boundary, world)
    });
    world.insert_resource(publications);
    result
}
