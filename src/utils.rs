pub fn sort_by_key_ref<T, B, F>(slice: &mut [T], mut f: F)
where
    F: FnMut(&T) -> &B,
    B: Ord,
{
    slice.sort_by(|a, b| f(a).cmp(f(b)))
}
