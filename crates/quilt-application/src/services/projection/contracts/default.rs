//! Default projection — re-export of the domain ZST.
//
// This module exists so the application layer can register `DefaultProjection`
// alongside the other V1 contracts in `StaticProjectionRegistry::v1()`.

pub use quilt_domain::projection::DefaultProjection;
