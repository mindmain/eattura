//! Invoice lifecycle state machine, shared between the server and the desktop app.
//!
//! The canonical issuing lifecycle is:
//!
//! ```text
//! draft ──▶ validated ──▶ sent ──▶ delivered ──▶ accepted
//!   ▲           │           │          └───────▶ rejected
//!   │           │           ├──────────────────▶ accepted
//!   │           │           └──────────────────▶ rejected
//!   └───────────┘ (revert to edit)
//! ```
//!
//! `delivered` corresponds to SDI `RicevutaConsegna` (`RC`), which is terminal
//! for B2B/B2C invoices and an intermediate milestone for PA invoices awaiting
//! `NotificaEsito` (`NE`) or `NotificaDecorrenzaTermini` (`DT`).
//! `rejected` covers both SDI pre-delivery discards (`NS`, which can be reverted
//! to `draft` and re-sent) and post-delivery PA refusals (`NE EC02`). `imported`
//! invoices ingested from external XML can also be validated or transitioned as
//! SDI outcomes arrive.

use std::fmt;
use std::str::FromStr;

/// Lifecycle state of an invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoiceStatus {
    /// Editable working copy.
    Draft,
    /// Passed SDI validation, ready to send.
    Validated,
    /// Sent to SDI via PEC, awaiting outcome.
    Sent,
    /// Delivered to the recipient by SDI (ricevuta di consegna / RC).
    Delivered,
    /// Accepted by recipient or deemed accepted (notifica esito positivo / decorrenza termini).
    Accepted,
    /// Rejected by SDI or PA recipient (notifica di scarto / esito negativo).
    Rejected,
    /// Ingested from an external XML.
    Imported,
}

impl InvoiceStatus {
    /// The string form persisted in the database (lowercase).
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Validated => "validated",
            InvoiceStatus::Sent => "sent",
            InvoiceStatus::Delivered => "delivered",
            InvoiceStatus::Accepted => "accepted",
            InvoiceStatus::Rejected => "rejected",
            InvoiceStatus::Imported => "imported",
        }
    }

    /// Whether a transition from `self` to `next` is allowed.
    pub fn can_transition_to(self, next: InvoiceStatus) -> bool {
        use InvoiceStatus::*;
        matches!(
            (self, next),
            (Draft, Validated)
                | (Validated, Draft)
                | (Validated, Sent)
                | (Sent, Delivered)
                | (Sent, Accepted)
                | (Sent, Rejected)
                | (Delivered, Accepted)
                | (Delivered, Rejected)
                | (Rejected, Draft)
                | (Imported, Validated)
                | (Imported, Sent)
                | (Imported, Delivered)
                | (Imported, Accepted)
                | (Imported, Rejected)
        )
    }

    /// Validate a transition, returning an error message on an illegal move.
    pub fn transition_to(self, next: InvoiceStatus) -> Result<InvoiceStatus, String> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(format!(
                "Illegal status transition: {} → {}",
                self.as_str(),
                next.as_str()
            ))
        }
    }
}

impl fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for InvoiceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(InvoiceStatus::Draft),
            "validated" => Ok(InvoiceStatus::Validated),
            "sent" => Ok(InvoiceStatus::Sent),
            "delivered" => Ok(InvoiceStatus::Delivered),
            "accepted" => Ok(InvoiceStatus::Accepted),
            "rejected" => Ok(InvoiceStatus::Rejected),
            "imported" => Ok(InvoiceStatus::Imported),
            other => Err(format!("Unknown invoice status: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_transitions() {
        assert!(InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Validated));
        assert!(InvoiceStatus::Validated.can_transition_to(InvoiceStatus::Sent));
        assert!(InvoiceStatus::Validated.can_transition_to(InvoiceStatus::Draft));
        assert!(InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Delivered));
        assert!(InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Accepted));
        assert!(InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Rejected));
        assert!(InvoiceStatus::Delivered.can_transition_to(InvoiceStatus::Accepted));
        assert!(InvoiceStatus::Delivered.can_transition_to(InvoiceStatus::Rejected));
        assert!(InvoiceStatus::Rejected.can_transition_to(InvoiceStatus::Draft));
        assert!(InvoiceStatus::Imported.can_transition_to(InvoiceStatus::Validated));
        assert!(InvoiceStatus::Imported.can_transition_to(InvoiceStatus::Delivered));
        assert!(InvoiceStatus::Imported.can_transition_to(InvoiceStatus::Accepted));
    }

    #[test]
    fn illegal_transitions() {
        assert!(!InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Sent));
        assert!(!InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Draft));
        assert!(!InvoiceStatus::Delivered.can_transition_to(InvoiceStatus::Draft));
        assert!(!InvoiceStatus::Accepted.can_transition_to(InvoiceStatus::Draft));
        assert!(!InvoiceStatus::Accepted.can_transition_to(InvoiceStatus::Delivered));
        assert!(!InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Draft));
    }

    #[test]
    fn roundtrip_str() {
        for s in [
            "draft",
            "validated",
            "sent",
            "delivered",
            "accepted",
            "rejected",
            "imported",
        ] {
            assert_eq!(s.parse::<InvoiceStatus>().unwrap().as_str(), s);
        }
        assert!("bogus".parse::<InvoiceStatus>().is_err());
    }
}
