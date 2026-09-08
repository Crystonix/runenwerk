//! Query-relative result semantics for the R3 exact surface protocol.
//!
//! The result owns only hit/miss meaning. A hit position is derived from the canonicalized query
//! ray and non-negative distance, so callers cannot publish a hit whose position contradicts the
//! query. Dispatch/provider topology remains outside R3.

use super::representation::{RenderRepresentationValidationError, RenderSurfaceQuery};
use super::space_time::CanonicalF64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderSurfaceHit {
    distance_meters: CanonicalF64,
    position_scene_meters: [CanonicalF64; 3],
}

impl RenderSurfaceHit {
    fn from_query_distance(
        query: RenderSurfaceQuery,
        distance_meters: f64,
    ) -> Result<Self, RenderRepresentationValidationError> {
        let distance_meters = CanonicalF64::new(distance_meters, "surface_hit_distance_meters")?;
        if distance_meters.get() < 0.0 {
            return Err(RenderRepresentationValidationError::NegativeSurfaceHitDistance);
        }
        let hit = RenderSurfaceHit::from_query_distance(query, distance_meters)?;
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
