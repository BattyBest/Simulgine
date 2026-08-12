use ordered_float::NotNan;
use std::{
    collections::HashMap,
    hash::Hash,
    io::{BufWriter, Write},
};

use super::ast::*;

#[derive(Debug)]
pub struct UserClassMember {
    pub(super) name: String,
    pub(super) class: UserClassIndx,
    pub(super) t: TypeIdentifier,
    pub(super) body: Option<FASTNode>,
    pub(super) stage: NotNan<f64>,
    pub(super) access_level: FieldAccessLevel,
    pub(super) change_level: FieldChangeLevel,
}

impl PartialEq for UserClassMember {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for UserClassMember {}

impl Hash for UserClassMember {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug)]
pub struct UserClass {
    pub(super) name: String,
    pub(super) field_names: HashMap<String, usize>,
    pub(super) field_stages: Vec<Vec<usize>>,
    pub(super) fields: Vec<UserClassMember>,
}

impl UserClass {
    pub fn to_debug_string(&self, sim: &Simulgine) -> String {
        let mut buf = BufWriter::new(Vec::new());
        buf.write_all(self.name.as_bytes()).unwrap();
        buf.write_all(" {\n".as_bytes()).unwrap();

        for f in &self.fields {
            writeln!(buf, "\t{}: {}", f.name, f.t.to_debug_string(Some(sim))).unwrap();
        }

        buf.write_all("}".as_bytes()).unwrap();

        String::from_utf8(buf.into_inner().unwrap()).unwrap()
    }

    pub fn to_string(&self, _sim: &Simulgine) -> String {
        self.name.clone()
    }
}

#[derive(Debug)]
pub struct Simulgine {
    pub(super) user_class_names: HashMap<String, UserClassIndx>,
    pub(super) user_classes: Vec<UserClass>,
}

impl Simulgine {
    pub fn get_user_class(&self, i: UserClassIndx) -> Option<&UserClass> {
        self.user_classes.get(i.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UserClassIndx(pub(super) usize);

#[derive(Debug)]
pub struct SimulgineInst<'a> {
    pub(crate) based: &'a Simulgine,
    pub(crate) root: UserObject,
}

#[derive(Clone, Debug)]
pub struct UserObject {
    pub(crate) class: UserClassIndx,
    pub(crate) fields: Vec<TypeInstance>,
}

impl UserObject {
    pub(crate) fn spawn_user_object(sim: &Simulgine, t: UserClassIndx) -> Option<UserObject> {
        let mut ret = UserObject {
            class: t,
            fields: Vec::new(),
        };

        for a in sim.get_user_class(t)?.fields.iter() {
            ret.fields.push(spawn_type_instance(sim, &a.t));
        }

        Some(ret)
    }

    pub fn to_debug_string(&self, sim: &Simulgine) -> String {
        let cl = sim.get_user_class(self.class);

        if let Some(class) = cl {
            let mut buf = BufWriter::new(Vec::new());
            buf.write_all(class.name.as_bytes()).unwrap();
            buf.write_all(" {\n".as_bytes()).unwrap();

            let mut i: usize = 0;
            while i < self.fields.len() {
                let f = &class.fields[i];
                writeln!(
                    buf,
                    "\t{}: {} = {}",
                    f.name,
                    f.t.to_string(Some(sim)),
                    self.fields[i].to_string(Some(sim)),
                )
                .unwrap();
                i += 1;
            }

            buf.write_all("}".as_bytes()).unwrap();

            String::from_utf8(buf.into_inner().unwrap()).unwrap()
        } else {
            "<invalid>".to_owned()
        }
    }

    pub fn to_string(&self, sim: &Simulgine) -> String {
        sim.get_user_class(self.class)
            .map_or("<invalid>".to_owned(), |x| x.to_string(sim))
    }
}

impl PartialEq for UserObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub(crate) fn spawn_type_instance_const(t: &TypeIdentifier) -> TypeInstance {
    match t {
        TypeIdentifier::UserClass(_) => TypeInstance::None,
        TypeIdentifier::UnlinkedType(_) => TypeInstance::None,
        TypeIdentifier::Type(type_identifier) => TypeInstance::Type(TypeInst {
            val: *(type_identifier.clone()),
        }),
        TypeIdentifier::Error => TypeInstance::None,
        TypeIdentifier::I64 => TypeInstance::I64(I64Inst { val: 0 }),
        TypeIdentifier::I32 => TypeInstance::I32(I32Inst { val: 0 }),
        TypeIdentifier::I16 => TypeInstance::I16(I16Inst { val: 0 }),
        TypeIdentifier::I8 => TypeInstance::I8(I8Inst { val: 0 }),
        TypeIdentifier::U64 => TypeInstance::U64(U64Inst { val: 0 }),
        TypeIdentifier::U32 => TypeInstance::U32(U32Inst { val: 0 }),
        TypeIdentifier::U16 => TypeInstance::U16(U16Inst { val: 0 }),
        TypeIdentifier::U8 => TypeInstance::U8(U8Inst { val: 0 }),
        TypeIdentifier::Float => TypeInstance::Float(FloatInst { val: 0.0 }),
        TypeIdentifier::Double => TypeInstance::Double(DoubleInst { val: 0.0 }),
        TypeIdentifier::String => TypeInstance::String(StringInst { val: "".to_owned() }),
        TypeIdentifier::Boolean => TypeInstance::Boolean(BooleanInst { val: false }),
        TypeIdentifier::None => TypeInstance::None,
    }
}

pub(crate) fn spawn_type_instance(sim: &Simulgine, t: &TypeIdentifier) -> TypeInstance {
    match t {
        TypeIdentifier::UserClass(weak) => TypeInstance::UserClass(UserClassInst {
            val: UserObject::spawn_user_object(sim, *weak).unwrap(),
        }),
        TypeIdentifier::UnlinkedType(_) => TypeInstance::None,
        TypeIdentifier::Type(type_identifier) => TypeInstance::Type(TypeInst {
            val: *(type_identifier.clone()),
        }),
        TypeIdentifier::Error => TypeInstance::None,
        TypeIdentifier::I64 => TypeInstance::I64(I64Inst { val: 0 }),
        TypeIdentifier::I32 => TypeInstance::I32(I32Inst { val: 0 }),
        TypeIdentifier::I16 => TypeInstance::I16(I16Inst { val: 0 }),
        TypeIdentifier::I8 => TypeInstance::I8(I8Inst { val: 0 }),
        TypeIdentifier::U64 => TypeInstance::U64(U64Inst { val: 0 }),
        TypeIdentifier::U32 => TypeInstance::U32(U32Inst { val: 0 }),
        TypeIdentifier::U16 => TypeInstance::U16(U16Inst { val: 0 }),
        TypeIdentifier::U8 => TypeInstance::U8(U8Inst { val: 0 }),
        TypeIdentifier::Float => TypeInstance::Float(FloatInst { val: 0.0 }),
        TypeIdentifier::Double => TypeInstance::Double(DoubleInst { val: 0.0 }),
        TypeIdentifier::String => TypeInstance::String(StringInst { val: "".to_owned() }),
        TypeIdentifier::Boolean => TypeInstance::Boolean(BooleanInst { val: false }),
        TypeIdentifier::None => TypeInstance::None,
    }
}
