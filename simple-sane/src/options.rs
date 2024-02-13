use crate::{ffi, result::from_status, SaneError, Scanner};
use bitflags::bitflags;
use bstr::BStr;
use std::{
    ffi::{c_void, CStr, CString},
    fmt::Debug,
    mem,
    ops::{Deref, RangeInclusive},
    ptr::null_mut,
};

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ScannerOptions<'sane, 'scanner>(Vec<ScannerOption<'sane, 'scanner>>);

#[derive(Clone)]
pub struct ScannerOption<'sane, 'scanner> {
    scanner: &'scanner Scanner<'sane>,
    pub number: i32,
    pub name: Option<&'scanner BStr>,
    pub title: &'scanner BStr,
    pub description: &'scanner BStr,
    pub ty: Type,
    pub unit: Unit,
    pub capatibilities: Capatibilities,
    pub constraint: Constraint<'scanner>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum Type {
    Bool,
    Int,
    Fixed,
    String,
    Button,
    Group,
    Unsupported,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum Unit {
    None,
    Pixel,
    Bit,
    Mm,
    Dpi,
    Percent,
    Microsecond,
    Unsupported,
}

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct Capatibilities: u32 {
        const SoftSelect = ffi::SANE_CAP_SOFT_SELECT;
        const HardSelect = ffi::SANE_CAP_HARD_SELECT;
        const SoftDetect = ffi::SANE_CAP_SOFT_DETECT;
        const Emulated = ffi::SANE_CAP_EMULATED;
        const Automatic = ffi::SANE_CAP_AUTOMATIC;
        const Inactive = ffi::SANE_CAP_INACTIVE;
        const Advanced = ffi::SANE_CAP_ADVANCED;

        const _ = !0;
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Constraint<'a> {
    None,
    Range {
        range: RangeInclusive<i32>,
        quant: i32,
    },
    WordList {
        // TODO: Support constraint
    },
    StringList(Vec<&'a BStr>),
    Unsupported,
}

#[derive(Debug, Clone)]
pub enum Value<'a> {
    Bool(bool),
    Int(i32),
    String(&'a BStr),
}

impl<'sane, 'scanner> ScannerOptions<'sane, 'scanner> {
    pub fn new(scanner: &'scanner Scanner<'sane>) -> Self {
        let device_handle = unsafe { scanner.get_device_handle() };

        let mut options = Vec::new();

        let mut idx = 0;

        while let Some(option) = unsafe {
            log::trace!("Call ffi::sane_get_option_descriptor({device_handle:p}, {idx})");
            ffi::sane_get_option_descriptor(device_handle, idx).as_ref()
        } {
            options.push(ScannerOption::new(scanner, idx, option));
            idx += 1;
        }

        Self(options)
    }
}

impl<'sane, 'scanner> IntoIterator for ScannerOptions<'sane, 'scanner> {
    type Item = <Vec<ScannerOption<'sane, 'scanner>> as IntoIterator>::Item;
    type IntoIter = <Vec<ScannerOption<'sane, 'scanner>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'sane, 'scanner> ScannerOption<'sane, 'scanner> {
    pub fn new(
        scanner: &'scanner Scanner<'sane>,
        idx: i32,
        option: &'scanner ffi::SANE_Option_Descriptor,
    ) -> Self {
        Self {
            scanner,
            number: idx,
            name: unsafe { Self::cstr2bstr(option.name) },
            title: unsafe { Self::cstr2bstr(option.title) }.expect("title should be not null"),
            description: unsafe { Self::cstr2bstr(option.desc) }.expect("desc should be not null"),
            ty: option.type_.into(),
            unit: option.unit.into(),
            capatibilities: Capatibilities::from_bits_retain(unsafe { mem::transmute(option.cap) }),
            constraint: Constraint::new(option.constraint_type, option.constraint),
        }
    }

    unsafe fn cstr2bstr<'a>(str: *const std::ffi::c_char) -> Option<&'a BStr> {
        str.as_ref()
            .map(|cstr| CStr::from_ptr(cstr).to_bytes().into())
    }

    pub fn is_settable(&self) -> bool {
        self.capatibilities.contains(Capatibilities::SoftSelect)
    }

    pub fn auto(&self) -> Result<(), SaneError> {
        from_status(unsafe {
            ffi::sane_control_option(
                self.scanner.get_device_handle(),
                self.number,
                ffi::SANE_Action_SANE_ACTION_SET_AUTO,
                null_mut(),
                null_mut(),
            )
        })
    }

    pub fn set_value(&self, value: Value) -> Result<(), SaneError> {
        match value {
            Value::Bool(_) => todo!(),
            Value::Int(_) => todo!(),
            Value::String(str) => {
                let str: &[u8] = str.as_ref();
                let cstr: CString = CString::new(str).map_err(|_| SaneError::Inval)?;

                let value = cstr.into_raw();
                self.control_option(ffi::SANE_Action_SANE_ACTION_SET_VALUE, value as *mut c_void);

                unsafe { CString::from_raw(value) };
            }
        }

        Ok(())
    }

    fn control_option(
        &self,
        action: ffi::SANE_Action,
        value: *mut c_void,
    ) -> Result<(), SaneError> {
        from_status(unsafe {
            ffi::sane_control_option(
                self.scanner.get_device_handle(),
                self.number,
                action,
                value,
                null_mut(),
            )
        })
    }
}

impl Debug for ScannerOption<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Option")
            .field("number", &self.number)
            .field("name", &self.name)
            .field("title", &self.title)
            .field("description", &self.description)
            .field("type", &self.ty)
            .field("unit", &self.unit)
            .field("capatibilities", &self.capatibilities)
            .field("constraint", &self.constraint)
            .finish()
    }
}

impl From<ffi::SANE_Value_Type> for Type {
    fn from(ty: ffi::SANE_Value_Type) -> Self {
        match ty {
            ffi::SANE_Value_Type_SANE_TYPE_BOOL => Self::Bool,
            ffi::SANE_Value_Type_SANE_TYPE_INT => Self::Int,
            ffi::SANE_Value_Type_SANE_TYPE_FIXED => Self::Fixed,
            ffi::SANE_Value_Type_SANE_TYPE_STRING => Self::String,
            ffi::SANE_Value_Type_SANE_TYPE_BUTTON => Self::Button,
            ffi::SANE_Value_Type_SANE_TYPE_GROUP => Self::Group,
            _ => Self::Unsupported,
        }
    }
}

impl From<ffi::SANE_Unit> for Unit {
    fn from(unit: ffi::SANE_Unit) -> Self {
        match unit {
            ffi::SANE_Unit_SANE_UNIT_NONE => Self::None,
            ffi::SANE_Unit_SANE_UNIT_PIXEL => Self::Pixel,
            ffi::SANE_Unit_SANE_UNIT_BIT => Self::Bit,
            ffi::SANE_Unit_SANE_UNIT_MM => Self::Mm,
            ffi::SANE_Unit_SANE_UNIT_DPI => Self::Dpi,
            ffi::SANE_Unit_SANE_UNIT_PERCENT => Self::Percent,
            ffi::SANE_Unit_SANE_UNIT_MICROSECOND => Self::Microsecond,
            _ => Self::Unsupported,
        }
    }
}

impl<'a> Constraint<'a> {
    fn new(
        ty: ffi::SANE_Constraint_Type,
        constraint: ffi::SANE_Option_Descriptor__bindgen_ty_1,
    ) -> Self {
        match ty {
            ffi::SANE_Constraint_Type_SANE_CONSTRAINT_NONE => Self::None,
            ffi::SANE_Constraint_Type_SANE_CONSTRAINT_RANGE => {
                let range = unsafe { *constraint.range };

                Self::Range {
                    range: RangeInclusive::new(range.min, range.max),
                    quant: range.quant,
                }
            }
            ffi::SANE_Constraint_Type_SANE_CONSTRAINT_WORD_LIST => {
                Self::WordList { /* TODO: Unsupported constraint */ }
            }
            ffi::SANE_Constraint_Type_SANE_CONSTRAINT_STRING_LIST => {
                let mut values = Vec::new();

                unsafe {
                    let mut string_list = constraint.string_list;
                    while let Some(value) = (*string_list).as_ref::<'a>() {
                        let value = CStr::from_ptr(value).to_bytes().into();
                        values.push(value);

                        string_list = string_list.add(1);
                    }
                }

                Self::StringList(values)
            }
            _ => Self::Unsupported,
        }
    }
}
