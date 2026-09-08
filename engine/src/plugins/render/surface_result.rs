//! Query-relative result semantics for the R3 exact surface protocol.
//!
//! The result owns only hit/miss meaning. A hit position is derived from the canonicalized query
//! ray and non-negative distance, so callers cannot publish a hit whose position contradicts the
//! query. Dispatch/provider topology remains outside R3.

use super::representation::{
    RenderRepresentationValidationError, RenderSurfaceHit, RenderSurfaceQuery,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderSurfaceQueryResult {
    kind: RenderSurfaceQueryResultKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RenderSurfaceQueryResultKind {
    Miss,
    Hit(RenderSurfaceHit),
}

impl RenderSurfaceQueryResult {
    pub const fn miss() -> Self {
        Self {
            kind: RenderSurfaceQueryResultKind::Miss,
        }
    }

    pub fn hit_at_distance(
        query: RenderSurfaceQuery,
        distance_meters: f64,
    ) -> Result<Self, RenderRepresentationValidationError> {
        let origin = query.origin_scene_meters();
        let direction = query.direction_scene();
        let position = [
            origin[0] + direction[0] * distance_meters,
            origin[1] + direction[1] * distance_meters,
            origin[2] + direction[2] * distance_meters,
        ];
        let hit = RenderSurfaceHit::new(distance_meters, position)?;
        Ok(Self {
            kind: RenderSurfaceQueryResultKind::Hit(hit),
        })
    }

    pub const fn is_miss(self) -> bool {
        matches!(self.kind, RenderSurfaceQueryResultKind::Miss)
    }

    pub const fn hit(self) -> Option<RenderSurfaceHit> {
        match self.kind {
            RenderSurfaceQueryResultKind::Miss => None,
            RenderSurfaceQueryResultKind::Hit(hit) => Some(hit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::space_time::RenderTimePoint;

    fn query() -> RenderSurfaceQuery {
        RenderSurfaceQuery::new(
            [0.0, 0.0, 3.0],
            [0.0, 0.0, -2.0],
            RenderTimePoint::from_seconds(0.0).expect("finite time"),
        )
        .expect("valid query")
    }

    #[test]
    fn surface_result_has_explicit_miss_semantics() {
        let result = RenderSurfaceQueryResult::miss();
        assert!(result.is_miss());
        assert_eq!(result.hit(), None);
    }

    #[test]
    fn surface_hit_position_is_derived_from_query_ray() {
        let result = RenderSurfaceQueryResult::hit_at_distance(query(), 2.0)
            .expect("non-negative hit distance");
        let hit = result.hit().expect("hit result");
        assert_eq!(hit.distance_meters(), 2.0);
        assert_eq!(hit.position_scene_meters(), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn surface_result_rejects_negative_hit_distance() {
        assert_eq!(
            RenderSurfaceQueryResult::hit_at_distance(query(), -1.0),
            Err(RenderRepresentationValidationError::NegativeSurfaceHitDistance)
        );
    }
}
