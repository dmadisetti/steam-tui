macro_rules! log{
    ($($elem: expr),+ ) => {
        if !atty::is(atty::Stream::Stderr) {
            eprintln!(concat!($(concat!(stringify!($elem), " - {:?}\n")),+), $($elem),+);
        }
    };
}

pub(crate) use log;
