#[cfg(test)]
mod tests {
    use al_core::{EventMarker, EventRequirements};
    use al_derive::event;

    /// Helper function to ensure a type implements EventMarker
    fn has_impl_marker<T: EventMarker>() {}

    /// Test `event` attribute and `EventMarker` derive macros for simple structs
    #[test]
    fn event_marker_derive() {
        // Using `event` attribute macro
        #[event]
        struct TestEventA;

        // Using `event` attribute macro with existing derive
        // The `#[event]` macro will duplicate derives if after any `#derive(...)]`
        #[event]
        #[derive(Clone)]
        struct TestEventB(String, Vec<u128>);

        // Using `EventMarker` derive macro
        #[derive(Clone, Default, PartialEq, Hash, Debug, al_derive::EventMarker)]
        struct TestEventC {
            x: u8,
            y: u8,
        }

        has_impl_marker::<TestEventA>();
        has_impl_marker::<TestEventB>();
        has_impl_marker::<TestEventC>();
    }

    /// Test `event` attribute and `EventMarker` derive macros with generics
    #[test]
    fn generic_marker_derive() {
        // Using `event` attribute macro
        #[event]
        struct GenericEvent<T>(T);
        has_impl_marker::<GenericEvent<u128>>();
        has_impl_marker::<GenericEvent<String>>();

        // This should fail to compile if uncommented, as GenericType does not implement EventRequirements
        //struct GenericType;
        //has_impl_marker::<GenericEvent<GenericType>>();

        // Using `EventMarker` derive macro
        #[derive(Clone, Default, PartialEq, Hash, Debug, al_derive::EventMarker)]
        struct GenericEvent2<T>(T);
        has_impl_marker::<GenericEvent2<u128>>();
        has_impl_marker::<GenericEvent2<String>>();
    }
}
