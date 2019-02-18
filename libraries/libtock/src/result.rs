use core::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(isize)]
pub enum Error {
    FAIL = -1,
    EBUSY = -2,
    EALREADY = -3,
    EOFF = -4,
    ERESERVE = -5,
    EINVAL = -6,
    ESIZE = -7,
    ECANCEL = -8,
    ENOMEM = -9,
    ENOSUPPORT = -10,
    ENODEVICE = -11,
    EUNINSTALLED = -12,
    ENOACK = -13,
}

impl From<isize> for Error {
    fn from(i: isize) -> Error {
        match i {
            -1 => Error::FAIL,
            -2 => Error::EBUSY,
            -3 => Error::EALREADY,
            -4 => Error::EOFF,
            -5 => Error::ERESERVE,
            -6 => Error::EINVAL,
            -7 => Error::ESIZE,
            -8 => Error::ECANCEL,
            -9 => Error::ENOMEM,
            -10 => Error::ENOSUPPORT,
            -11 => Error::ENODEVICE,
            -12 => Error::EUNINSTALLED,
            -13 => Error::ENOACK,
            _ => panic!("invalid isize"),
        }
    }
}

// `newtype` to take care of errors returned via `usize` with a `isize` value.
pub struct UsizeError(pub Option<Error>);

impl From<usize> for UsizeError {
    fn from(u: usize) -> UsizeError {
        match u {
            fail if Error::FAIL as usize == fail => UsizeError(Some(Error::FAIL)),
            ebusy if Error::EBUSY as usize == ebusy => UsizeError(Some(Error::EBUSY)),
            ealready if Error::EALREADY as usize == ealready => UsizeError(Some(Error::EALREADY)),
            eoff if Error::EOFF as usize == eoff => UsizeError(Some(Error::EOFF)),
            ereserve if Error::ERESERVE as usize == ereserve => UsizeError(Some(Error::ERESERVE)),
            einval if Error::EINVAL as usize == einval => UsizeError(Some(Error::EINVAL)),
            esize if Error::ESIZE as usize == esize => UsizeError(Some(Error::ESIZE)),
            ecancel if Error::ECANCEL as usize == ecancel => UsizeError(Some(Error::ECANCEL)),
            enomem if Error::ENOMEM as usize == enomem => UsizeError(Some(Error::ENOMEM)),
            enosupport if Error::ENOSUPPORT as usize == enosupport => {
                UsizeError(Some(Error::ENOSUPPORT))
            }
            enodevice if Error::ENODEVICE as usize == enodevice => {
                UsizeError(Some(Error::ENODEVICE))
            }
            euninstalled if Error::EUNINSTALLED as usize == euninstalled => {
                UsizeError(Some(Error::EUNINSTALLED))
            }
            enoack if Error::ENOACK as usize == enoack => UsizeError(Some(Error::ENOACK)),
            _ => UsizeError(None),
        }
    }
}
