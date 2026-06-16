//! Invoice lifecycle state machine, shared between the server and the desktop app.
//!
//! The canonical issuing lifecycle is:
//!
//! ```text
//! draft ──▶ validated ──▶ sent ──▶ accepted
//!   ▲           │                └─▶ rejected
//!   └───────────┘ (revert to edit)
//! ```
//!
//! `rejected` invoices (SDI "scarto" / NS) can be reverted to `draft` to be
//! corrected and re-sent. `imported` is a separate received state used for
//! invoices ingested from XML and is not part of the issuing flow.

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
    /// Accepted by SDI (ricevuta di consegna / esito positivo).
    Accepted,
    /// Rejected by SDI (notifica di scarto / esito negativo).
    Rejected,
    /// Ingested from an external XML, outside the issuing flow.
    Imported,
}

impl InvoiceStatus {
    /// The string form persisted in the database (lowercase).
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Validated => "validated",
            InvoiceStatus::Sent => "sent",
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
                | (Sent, Accepted)
                | (Sent, Rejected)
                | (Rejected, Draft)
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
        assert!(InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Accepted));
        assert!(InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Rejected));
        assert!(InvoiceStatus::Rejected.can_transition_to(InvoiceStatus::Draft));
    }

    #[test]
    fn illegal_transitions() {
        assert!(!InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Sent));
        assert!(!InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Draft));
        assert!(!InvoiceStatus::Accepted.can_transition_to(InvoiceStatus::Draft));
        assert!(!InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Draft));
    }

    #[test]
    fn roundtrip_str() {
        for s in ["draft", "validated", "sent", "accepted", "rejected", "imported"] {
            assert_eq!(s.parse::<InvoiceStatus>().unwrap().as_str(), s);
        }
        assert!("bogus".parse::<InvoiceStatus>().is_err());
    }
}
