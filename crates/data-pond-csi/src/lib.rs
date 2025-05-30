#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::doc_overindented_list_items)]

pub mod csi {
    ::tonic::include_proto!("csi.v1");
}

pub mod pond {
    ::tonic::include_proto!("io.ulagbulag.csi.pond.v1");

    impl self::device_layer::Type {
        /// Return the child margin size used by layer.
        /// (e.g. VG -> LV overhead)
        ///
        #[inline]
        pub const fn margin(&self) -> i64 {
            match self {
                Self::Unknown => 0,
                Self::Lvm => 0,
            }
        }

        /// Return the parent padding size used by layer.
        /// (e.g. LV -> PV -> VG overhead)
        ///
        #[inline]
        pub const fn padding(&self) -> i64 {
            match self {
                Self::Unknown => 0,
                Self::Lvm => 4 << 20, // LVM metadata & alignment overhead
            }
        }
    }
}
