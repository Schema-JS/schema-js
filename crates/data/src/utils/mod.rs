pub mod fs;
pub mod hash;

pub(crate) fn flatten(input: &[&[u8]]) -> Vec<u8> {
    let total_len = input.iter().map(|slice| slice.len()).sum();
    let mut buffer = Vec::with_capacity(total_len);

    for slice in input {
        buffer.extend_from_slice(slice);
    }

    buffer
}
