pub mod sdi {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/eattura.sdi.v1.rs"));
    }
}

pub mod error;
pub mod invoice_status;
pub mod invoice_total;
pub mod p7m;
pub mod pdf;
pub mod pec;
pub mod validation;
pub mod vies;
pub mod xml;
