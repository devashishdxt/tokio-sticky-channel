use std::{
    hash::{BuildHasher, Hash},
    num::TryFromIntError,
};

pub fn compute_route_id<ID, S>(
    id: ID,
    num_consumers: usize,
    build_hasher: &S,
) -> Result<usize, TryFromIntError>
where
    ID: Hash,
    S: BuildHasher,
{
    let hash = usize::try_from(build_hasher.hash_one(id))?;
    Ok(hash % num_consumers)
}
