use crate::log;

pub fn rule(desc: &str) {
    log!("Rule: {}", desc);
}

pub fn example(from: &str, into: &str) {
    log!("Example: {} --> {}", from, into);
}
