pub mod sdi {

    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/eattura.sdi.v1.rs"));
    }
}
