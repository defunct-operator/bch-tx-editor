pub trait StrEnum: Sized {
    fn to_str(self) -> &'static str;
    fn from_str(s: &str) -> Option<Self>;
}

macro_rules! str_enum {
    ($( #[$attrs:meta] )* $vis:vis enum $name:ident {
        $( $( #[$variant_attrs:meta] )* $variant:ident = $str_value:literal ),* $(,)?
    }) => {
        $(#[$attrs])* $vis enum $name {
            $(
                $(#[$variant_attrs])* $variant
            ),*
        }
        impl $crate::macros::StrEnum for $name {
            fn to_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $str_value,)*
                }
            }

            fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($str_value => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    }
}

pub struct DropGuard<F: FnOnce()> {
    on_drop: Option<F>,
}

impl<F: FnOnce()> DropGuard<F> {
    pub fn new(on_drop: F) -> Self {
        Self {
            on_drop: Some(on_drop),
        }
    }

    pub fn disarm(mut self) {
        self.on_drop = None;
    }
}

impl<F: FnOnce()> Drop for DropGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.on_drop.take() {
            f();
        }
    }
}
