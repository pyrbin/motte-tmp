//! Modified from [Bevy's CI runner](https://github.com/Leafwing-Studios/template-repo/blob/main/tools/ci/src/main.rs)

use bitflags::bitflags;
use xshell::{cmd, Shell};

bitflags! {
    struct Check: u32 {
        const FORMAT = 0b00000001;
        const CLIPPY = 0b00000010;
        const COMPILE_CHECK = 0b100000000;
    }
}

// This can be configured as needed
const CLIPPY_FLAGS: [&str; 2] = ["-Aclippy::type_complexity", "-Dwarnings"];

fn main() {
    let arguments = [
        ("lints", Check::FORMAT | Check::CLIPPY),
        ("compile", Check::COMPILE_CHECK),
        ("format", Check::FORMAT),
        ("clippy", Check::CLIPPY),
    ];

    let what_to_run = if let Some(arg) = std::env::args().nth(1).as_deref() {
        if let Some((_, check)) = arguments.iter().find(|(str, _)| *str == arg) {
            *check
        } else {
            println!(
                "Invalid argument: {arg:?}.\nEnter one of: {}.",
                arguments[1..].iter().map(|(s, _)| s).fold(arguments[0].0.to_owned(), |c, v| c + ", " + v)
            );
            return;
        }
    } else {
        Check::all()
    };

    let sh = Shell::new().unwrap();

    if what_to_run.contains(Check::FORMAT) {
        // See if any code needs to be formatted
        cmd!(sh, "cargo fmt --all -- --check").run().expect("Please run 'cargo fmt --all' to format your code.");
    }

    if what_to_run.contains(Check::CLIPPY) {
        cmd!(sh, "cargo clippy --workspace --all-features -- {CLIPPY_FLAGS...}")
            .run()
            .expect("Please fix clippy errors in output above.");
    }

    if what_to_run.contains(Check::COMPILE_CHECK) {
        cmd!(sh, "cargo c --workspace").run().expect("Please fix compiler errors in above output.");
    }
}
