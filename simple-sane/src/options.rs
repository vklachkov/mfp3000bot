use crate::{ffi, result::from_status, utils::cstr2bstr, SaneError, Scanner};
use bitflags::bitflags;
use bstr::BStr;
use std::{
    ffi::{c_void, CStr, CString},
    fmt::Debug,
    mem,
    ops::{Deref, RangeInclusive},
    ptr::{self, null, null_mut},
};

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ScannerOptions<'b, 'd>(Vec<ScannerOption<'b, 'd>>);

#[derive(Clone)]
pub struct ScannerOption<'b, 'd> {
    scanner: &'d Scanner<'b>,

    pub number: i32,
    pub name: Option<&'d BStr>,
    pub title: &'d BStr,
    pub description: &'d BStr,
    pub ty: Type,
    pub unit: Unit,
    pub capatibilities: Capatibilities,
    pub constraint: Constraint<'d>,
}

#[derive(Debug, Clone, Copy)]
pub enum Type {
    Bool,
    Int,
    Fixed,
    String,
    Button,
    Group,
}

#[derive(Debug, Clone, Copy)]
pub enum Unit {
    None,
    Pixel,
    Bit,
    Mm,
    Dpi,
    Percent,
    Microsecond,
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
}

#[derive(Debug, Clone)]
pub enum Value<'a> {
    Bool(bool),
    Int(i32),
    String(&'a BStr),
}

impl<'b, 'd> ScannerOptions<'b, 'd> {
    pub(crate) fn new(scanner: &'d Scanner<'b>) -> Self {
        Self(
            (0i32..i32::MAX)
                .map_while(|i| Self::get_option(scanner, i))
                .collect(),
        )
    }

    fn get_option(scanner: &'d Scanner<'b>, i: i32) -> Option<ScannerOption<'b, 'd>> {
        let handle = unsafe { scanner.get_device_handle() };

        log::trace!("Call ffi::sane_get_option_descriptor({handle:p}, {i})");
        let desc = unsafe { ffi::sane_get_option_descriptor(handle, i).as_ref() }?;

        Some(ScannerOption::new(scanner, i, desc))
    }
}

impl<'b, 'd> IntoIterator for ScannerOptions<'b, 'd> {
    type Item = <Vec<ScannerOption<'b, 'd>> as IntoIterator>::Item;
    type IntoIter = <Vec<ScannerOption<'b, 'd>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'b, 'd> ScannerOption<'b, 'd> {
    pub fn new(scanner: &'d Scanner<'b>, i: i32, desc: &'d ffi::SANE_Option_Descriptor) -> Self {
        Self {
            scanner,
            number: i,
            name: unsafe { cstr2bstr(desc.name) },
            title: unsafe { cstr2bstr(desc.title) }.expect("title should be not null"),
            description: unsafe { cstr2bstr(desc.desc) }.expect("desc should be not null"),
            ty: desc.type_.into(),
            unit: desc.unit.into(),
            capatibilities: Capatibilities::from_bits_retain(unsafe { mem::transmute(desc.cap) }),
            constraint: Constraint::new(desc.constraint_type, desc.constraint),
        }
    }

    pub fn is_settable(&self) -> bool {
        self.capatibilities.contains(Capatibilities::SoftSelect)
    }

    pub fn is_auto_settable(&self) -> bool {
        self.capatibilities.contains(Capatibilities::Automatic)
    }

    pub fn set_value(&self, value: Value) -> Result<(), SaneError> {
        // TODO: Check constraints.

        match value {
            Value::Bool(bool) => {
                let value = &bool as *const bool as *mut c_void;
                self.control_option(ffi::SANE_Action_SANE_ACTION_SET_VALUE, value);
            }
            Value::Int(int) => {
                let bytes = int.to_ne_bytes();

                let value = &bytes as *const u8 as *mut c_void;
                self.control_option(ffi::SANE_Action_SANE_ACTION_SET_VALUE, value);
            }
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

    pub fn set_auto(&self) -> Result<(), SaneError> {
        self.control_option(ffi::SANE_Action_SANE_ACTION_SET_AUTO, null_mut())
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
            _ => unreachable!(),
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
            _ => unreachable!(),
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
                let mut values = (0..usize::MAX)
                    .map_while(|offset| unsafe { cstr2bstr(*constraint.string_list.add(offset)) })
                    .collect();

                Self::StringList(values)
            }
            _ => unreachable!(),
        }
    }
}
