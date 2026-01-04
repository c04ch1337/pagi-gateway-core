use hyper::Request;

/// JWT auth placeholder.
///
/// For MVP this accepts missing/invalid tokens; hook your validator here.
pub fn authorize<B>(_req: &Request<B>) -> bool {
    true
}

