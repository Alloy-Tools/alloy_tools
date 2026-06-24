// ----- AsAny -----
pub trait AsAny: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn as_any_box(self) -> Box<dyn std::any::Any>;
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
}

impl<T: 'static> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_any_box(self) -> Box<dyn std::any::Any> {
        Box::new(self)
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ----- Downcast -----
pub trait Downcast {
    /// Downcast to a concrete `T`.
    fn downcast<T: 'static>(self) -> Result<T, Box<dyn std::any::Any>>;

    /// Downcast to a concrete `&T`.
    fn downcast_ref<T: 'static>(&self) -> Option<&T>;

    /// Downcast to a concrete `&mut T`.
    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T>;

    /// Downcast to a `Box<T>`. Returns `Err(Box<Self>)` if the type does not match.
    fn downcast_box<T: 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>>;
}

impl<F: AsAny + Sized> Downcast for F {
    fn downcast<T: 'static>(self) -> Result<T, Box<dyn std::any::Any>> {
        self.as_any_box().downcast::<T>().map(|boxed| *boxed)
    }

    fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }

    fn downcast_box<T: 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if self.as_any().is::<T>() {
            let raw = Box::into_raw(self);
            // SAFETY: type id checked above.
            Ok(unsafe { Box::from_raw(raw as *mut T) })
        } else {
            Err(self)
        }
    }
}

// ----- Type name -----
pub trait TypeName {
    fn module_path() -> &'static str;
    /// Helper function to return the simple names of generic types
    fn type_with_generics() -> String {
        format!("{}::{}", Self::module_path(), tynm::type_name::<Self>())
    }
}
pub trait DynTypeName {
    fn module_path(&self) -> &'static str;
    /// Helper function to return the simple names of generic types
    fn type_with_generics(&self) -> String;
}
impl<T: TypeName> DynTypeName for T {
    fn module_path(&self) -> &'static str {
        T::module_path()
    }

    fn type_with_generics(&self) -> String {
        T::type_with_generics()
    }
}
