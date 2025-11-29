//! Directories for ME generation 2 and 3
//!
//! There are multiple kinds of partitioning schemes, and some partitions may
//! contain directories, but directories could also be referenced by other data
//! structures, such as in the case of IFWI, so they are separate here.

pub mod gen2;
pub mod gen3;
pub mod gen3_man;
pub mod man;
