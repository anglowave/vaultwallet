use rand::rngs::OsRng;
use rand::RngCore;

pub fn random_bytes(out: &mut [u8]) {
	OsRng.fill_bytes(out);
}
