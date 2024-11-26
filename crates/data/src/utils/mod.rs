use std::marker::PhantomData;

pub mod fs;
pub mod hash;

pub(crate) fn flatten_with_callback<'a, F>(input: &'a [&'a [u8]], process_cb: Option<F>) -> Vec<u8>
where
    F: Fn(&'a [u8]) -> Vec<u8>,
{
    let mut buffer = Vec::new();
    let _marker: PhantomData<F> = PhantomData; // Helps compiler with `None` type inference

    for &slice in input {
        match &process_cb {
            Some(cb) => {
                let processed_slice = cb(slice);
                buffer.extend_from_slice(&processed_slice); // Borrow Vec<u8> output as &[u8]
            }
            None => {
                buffer.extend_from_slice(slice);
            }
        }
    }

    buffer
}

pub(crate) fn flatten(input: Vec<&[u8]>) -> Vec<u8> {
    let total_len = input.iter().map(|slice| slice.len()).sum();
    let mut buffer = Vec::with_capacity(total_len);

    for slice in input {
        buffer.extend_from_slice(slice);
    }

    buffer
}

pub fn is_zero_aligned(buf: &[u8]) -> bool {
    let (prefix, aligned, suffix) = unsafe { buf.align_to::<u128>() };

    prefix.iter().all(|&x| x == 0)
        && suffix.iter().all(|&x| x == 0)
        && aligned.iter().all(|&x| x == 0)
}
