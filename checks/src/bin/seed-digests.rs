//! Prints `<seed> <sha256>` for every reference seed. CI runs this natively on
//! Linux, macOS and Windows and as WebAssembly, and fails if any output differs.

fn main() {
    print!("{}", automark_checks::digest_table());
}
