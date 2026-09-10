use runen_net::identity::{ConnectionHandle, ParticipantId};
use runen_net::protocol::{NegotiationManager, NegotiationManagerError};
use runen_net::session::{
    ConnectionLossOutcome, MembershipState, RecoveryTime, RetentionPolicy, Session, SessionError,
};
use std::collections::HashMap;

/// Read-only engine projection of bindings already accepted by RunenNet [`Session`].
///
/// This projection exists because the accepted Core intentionally does not expose membership
/// iteration. It is updated only after successful Core lifecycle operations and is suitable for
/// engine scheduling, owner routing, diagnostics, and presentation. It is never consulted to
/// authorize admission, loss, retention, replacement, removal, expiry, or closure.
#[derive(Debug, Clone, Default, PartialEq, Eq, runen_ecs::Component, runen_ecs::Resource)]
pub struct RunenNetSessionProjection {
    bindings: HashMap<ConnectionHandle, ParticipantId>,
}

impl RunenNetSessionProjection {
    pub fn active_connections(&self) -> impl Iterator<Item = ConnectionHandle> + '_ {
        self.bindings.keys().copied()
    }

    pub fn participant_for_connection(
        &self,
        connection: ConnectionHandle,
    ) -> Option<ParticipantId> {
        self.bindings.get(&connection).copied()
    }

    pub fn active_connection_count(&self) -> usize {
        self.bindings.len()
    }

    fn record_binding(&mut self, connection: ConnectionHandle, participant: ParticipantId) {
        self.bindings.insert(connection, participant);
    }

    fn remove_binding(&mut self, connection: ConnectionHandle) {
        self.bindings.remove(&connection);
    }

    fn clear(&mut self) {
        self.bindings.clear();
    }
}

/// Engine-owned placement of the accepted RunenNet negotiation and session owners.
///
/// This resource does not redefine networking lifecycle semantics. RunenNet remains
/// authoritative for compatibility negotiation, participant membership, connection binding,
/// loss, retention, replacement, removal, expiry, and closure. Runenwerk owns only where these
/// Core owners live and when application lifecycle code invokes them.
#[derive(Debug, runen_ecs::Component, runen_ecs::Resource)]
pub struct RunenNetSessionCore {
    negotiation: NegotiationManager,
    session: Session,
}

impl RunenNetSessionCore {
    pub fn new(negotiation: NegotiationManager, session: Session) -> Self {
        Self {
            negotiation,
            session,
        }
    }

    pub fn negotiation(&self) -> &NegotiationManager {
        &self.negotiation
    }

    pub fn negotiation_mut(&mut self) -> &mut NegotiationManager {
        &mut self.negotiation
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn admit_established(
        &mut self,
        projection: &mut RunenNetSessionProjection,
        participant: ParticipantId,
        connection: ConnectionHandle,
    ) -> Result<(), RunenNetSessionCoreError> {
        let established = self
            .negotiation
            .established(connection)
            .map_err(RunenNetSessionCoreError::Negotiation)?;
        self.session
            .admit_new(participant, established)
            .map_err(RunenNetSessionCoreError::Session)?;
        projection.record_binding(connection, participant);
        Ok(())
    }

    pub fn bind_replacement(
        &mut self,
        projection: &mut RunenNetSessionProjection,
        participant: ParticipantId,
        connection: ConnectionHandle,
    ) -> Result<(), RunenNetSessionCoreError> {
        let established = self
            .negotiation
            .established(connection)
            .map_err(RunenNetSessionCoreError::Negotiation)?;
        self.session
            .bind_replacement(participant, established)
            .map_err(RunenNetSessionCoreError::Session)?;
        projection.record_binding(connection, participant);
        Ok(())
    }

    /// Apply RunenNet session loss policy and release the old connection's compatibility state.
    ///
    /// RunenNet's transport teardown releases connection-scoped negotiation state independently
    /// from the higher-level session retention decision. This Core-only engine composition does
    /// the same: the participant may remain retained, but the lost connection's established
    /// negotiation cannot remain live and a replacement must establish compatibility on a
    /// different [`ConnectionHandle`].
    pub fn connection_lost(
        &mut self,
        projection: &mut RunenNetSessionProjection,
        participant: ParticipantId,
        connection: ConnectionHandle,
        policy: RetentionPolicy,
    ) -> Result<ConnectionLossOutcome, RunenNetSessionCoreError> {
        let outcome = self
            .session
            .connection_lost(participant, connection, policy)
            .map_err(RunenNetSessionCoreError::Session)?;

        projection.remove_binding(connection);
        match self.negotiation.terminate(connection) {
            Ok(_) | Err(NegotiationManagerError::UnknownConnection) => Ok(outcome),
            Err(error) => Err(RunenNetSessionCoreError::NegotiationCleanup(error)),
        }
    }

    /// Remove membership through RunenNet while keeping connection teardown host-owned.
    pub fn remove_participant(
        &mut self,
        projection: &mut RunenNetSessionProjection,
        participant: ParticipantId,
    ) -> Result<MembershipState, RunenNetSessionCoreError> {
        let previous_state = self
            .session
            .remove_participant(participant)
            .map_err(RunenNetSessionCoreError::Session)?;
        if let MembershipState::Bound(connection) = previous_state {
            projection.remove_binding(connection);
        }
        Ok(previous_state)
    }

    /// Advance RunenNet's host-supplied recovery clock and return memberships expired by Core.
    pub fn advance_recovery_clock(
        &mut self,
        new_value: RecoveryTime,
    ) -> Result<Vec<ParticipantId>, RunenNetSessionCoreError> {
        self.session
            .advance_recovery_clock(new_value)
            .map_err(RunenNetSessionCoreError::Session)
    }

    /// Close session membership only.
    ///
    /// Transport/connection teardown remains a separate host responsibility. Clearing the
    /// projection prevents closed session membership from being treated as authorized engine
    /// routing state without pretending that `Session::close` also closes a transport.
    pub fn close(&mut self, projection: &mut RunenNetSessionProjection) {
        self.session.close();
        projection.clear();
    }

    pub fn participant_for_connection(
        &self,
        connection: ConnectionHandle,
    ) -> Option<ParticipantId> {
        self.session.participant_for_connection(connection)
    }

    pub fn membership_state(&self, participant: ParticipantId) -> Option<MembershipState> {
        self.session.membership_state(participant)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RunenNetSessionCoreError {
    Negotiation(NegotiationManagerError),
    Session(SessionError),
    NegotiationCleanup(NegotiationManagerError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_net::identity::SessionId;
    use runen_net::protocol::{
        CompatibilityOffer, NegotiatedContract, NegotiationManagerLimits, NegotiationRequirements,
        NegotiationStatus, OfferLimits, ProtocolContract, ProtocolId, ProtocolRevision,
    };
    use runen_net::session::{RecoveryDuration, SessionLimits};
    use std::num::{NonZeroU64, NonZeroUsize};

    fn protocol() -> ProtocolContract {
        ProtocolContract::new(ProtocolId::new(1), ProtocolRevision::new(1))
    }

    fn offer() -> CompatibilityOffer {
        CompatibilityOffer::new(vec![protocol()], vec![], vec![], None)
    }

    fn negotiation_manager() -> NegotiationManager {
        NegotiationManager::new(OfferLimits::default(), NegotiationManagerLimits::default())
            .expect("default RunenNet negotiation limits must be valid")
    }

    fn session() -> Session {
        let limit = NonZeroUsize::new(4).expect("non-zero session limit");
        let limits =
            SessionLimits::new(limit, limit).expect("matching session limits must be valid");
        Session::new(SessionId::new(1), limits)
    }

    fn establish(manager: &mut NegotiationManager, connection: ConnectionHandle) {
        manager
            .start(connection, offer(), offer())
            .expect("compatible negotiation must start");
        manager
            .propose(
                connection,
                NegotiatedContract::new(protocol()),
                &NegotiationRequirements::default(),
            )
            .expect("compatible contract must be proposed");
        assert_ne!(
            manager
                .validate_authority(connection)
                .expect("authority validation must succeed"),
            NegotiationStatus::Established
        );
        assert_eq!(
            manager
                .validate_peer(connection)
                .expect("peer validation must succeed"),
            NegotiationStatus::Established
        );
    }

    #[test]
    fn admission_requires_runennet_established_negotiation() {
        let mut core = RunenNetSessionCore::new(negotiation_manager(), session());
        let mut projection = RunenNetSessionProjection::default();
        let participant = ParticipantId::new(1);
        let connection = ConnectionHandle::new(1);

        assert_eq!(
            core.admit_established(&mut projection, participant, connection),
            Err(RunenNetSessionCoreError::Negotiation(
                NegotiationManagerError::UnknownConnection,
            ))
        );
        assert_eq!(core.session().live_memberships(), 0);
        assert_eq!(projection.active_connection_count(), 0);
    }

    #[test]
    fn terminal_connection_loss_is_owned_by_runennet_session() {
        let connection = ConnectionHandle::new(1);
        let participant = ParticipantId::new(1);
        let mut negotiation = negotiation_manager();
        establish(&mut negotiation, connection);
        let mut core = RunenNetSessionCore::new(negotiation, session());
        let mut projection = RunenNetSessionProjection::default();

        core.admit_established(&mut projection, participant, connection)
            .expect("established connection must be admitted");
        assert_eq!(
            core.participant_for_connection(connection),
            Some(participant)
        );
        assert_eq!(
            projection.participant_for_connection(connection),
            Some(participant)
        );

        assert_eq!(
            core.connection_lost(
                &mut projection,
                participant,
                connection,
                RetentionPolicy::Terminate,
            ),
            Ok(ConnectionLossOutcome::Terminated)
        );
        assert_eq!(core.participant_for_connection(connection), None);
        assert_eq!(core.membership_state(participant), None);
        assert_eq!(projection.participant_for_connection(connection), None);
        assert_eq!(core.negotiation().established_connections(), 0);
    }

    #[test]
    fn connection_loss_is_idempotent_with_prior_negotiation_cleanup() {
        let connection = ConnectionHandle::new(1);
        let participant = ParticipantId::new(1);
        let mut negotiation = negotiation_manager();
        establish(&mut negotiation, connection);
        let mut core = RunenNetSessionCore::new(negotiation, session());
        let mut projection = RunenNetSessionProjection::default();

        core.admit_established(&mut projection, participant, connection)
            .expect("established connection must be admitted");
        core.negotiation_mut()
            .terminate(connection)
            .expect("host teardown may release negotiation first");

        assert_eq!(
            core.connection_lost(
                &mut projection,
                participant,
                connection,
                RetentionPolicy::Terminate,
            ),
            Ok(ConnectionLossOutcome::Terminated)
        );
        assert_eq!(core.membership_state(participant), None);
        assert_eq!(projection.participant_for_connection(connection), None);
    }

    #[test]
    fn retained_membership_rebinds_only_to_another_established_connection() {
        let old_connection = ConnectionHandle::new(1);
        let new_connection = ConnectionHandle::new(2);
        let participant = ParticipantId::new(1);
        let mut negotiation = negotiation_manager();
        establish(&mut negotiation, old_connection);
        establish(&mut negotiation, new_connection);
        let mut core = RunenNetSessionCore::new(negotiation, session());
        let mut projection = RunenNetSessionProjection::default();

        core.admit_established(&mut projection, participant, old_connection)
            .expect("old connection must be admitted");
        let duration = RecoveryDuration::new(NonZeroU64::new(1).expect("non-zero recovery span"));
        assert!(matches!(
            core.connection_lost(
                &mut projection,
                participant,
                old_connection,
                RetentionPolicy::RetainForRecovery { duration },
            ),
            Ok(ConnectionLossOutcome::Retained { .. })
        ));
        assert_eq!(projection.participant_for_connection(old_connection), None);
        assert_eq!(core.negotiation().established_connections(), 1);

        core.bind_replacement(&mut projection, participant, new_connection)
            .expect("new established connection must rebind retained membership");
        assert_eq!(core.participant_for_connection(old_connection), None);
        assert_eq!(
            core.participant_for_connection(new_connection),
            Some(participant)
        );
        assert_eq!(
            projection.participant_for_connection(new_connection),
            Some(participant)
        );
        assert_eq!(
            core.membership_state(participant),
            Some(MembershipState::Bound(new_connection))
        );
    }

    #[test]
    fn participant_removal_and_recovery_expiry_remain_runennet_owned() {
        let bound_connection = ConnectionHandle::new(1);
        let retained_connection = ConnectionHandle::new(2);
        let bound_participant = ParticipantId::new(1);
        let retained_participant = ParticipantId::new(2);
        let mut negotiation = negotiation_manager();
        establish(&mut negotiation, bound_connection);
        establish(&mut negotiation, retained_connection);
        let mut core = RunenNetSessionCore::new(negotiation, session());
        let mut projection = RunenNetSessionProjection::default();

        core.admit_established(&mut projection, bound_participant, bound_connection)
            .expect("bound participant must be admitted");
        core.admit_established(&mut projection, retained_participant, retained_connection)
            .expect("retained participant must be admitted");

        assert_eq!(
            core.remove_participant(&mut projection, bound_participant),
            Ok(MembershipState::Bound(bound_connection))
        );
        assert_eq!(
            projection.participant_for_connection(bound_connection),
            None
        );

        let duration = RecoveryDuration::new(NonZeroU64::new(2).expect("non-zero recovery span"));
        core.connection_lost(
            &mut projection,
            retained_participant,
            retained_connection,
            RetentionPolicy::RetainForRecovery { duration },
        )
        .expect("retained participant loss must succeed");
        assert_eq!(
            core.advance_recovery_clock(RecoveryTime::new(2)),
            Ok(vec![retained_participant])
        );
        assert_eq!(core.membership_state(retained_participant), None);
        assert_eq!(
            projection.participant_for_connection(retained_connection),
            None
        );
    }

    #[test]
    fn closing_core_clears_engine_projection_after_runennet_closure() {
        let connection = ConnectionHandle::new(1);
        let participant = ParticipantId::new(1);
        let mut negotiation = negotiation_manager();
        establish(&mut negotiation, connection);
        let mut core = RunenNetSessionCore::new(negotiation, session());
        let mut projection = RunenNetSessionProjection::default();

        core.admit_established(&mut projection, participant, connection)
            .expect("established connection must be admitted");
        core.close(&mut projection);

        assert_eq!(core.participant_for_connection(connection), None);
        assert_eq!(projection.active_connection_count(), 0);
        assert_eq!(core.negotiation().established_connections(), 1);
    }
}
