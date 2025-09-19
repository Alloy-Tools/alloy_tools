#[cfg(test)]
mod tests {
    use al_core::{EventMarker, EventRequirements};
    use al_derive::{event, show_streams, EventMarker};

    /// Helper function to ensure a type implements EventMarker
    fn has_impl_marker<T: EventMarker>() {}

    /// Test deriving EventMarker for simple structs
    #[test]
    fn event_marker_derive() {
        #[allow(unused)]
        #[event]
        #[derive(Clone)]
        struct EventTestA;
        //has_impl_marker::<EventTestA>();

        #[derive(Clone)]
        #[show_streams]
        #[allow(unused)]
        struct EventTestB;

        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct TestEventA;

        has_impl_marker::<TestEventA>();

        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct TestEventB(String, Vec<u128>);

        has_impl_marker::<TestEventB>();

        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct TestEventC {
            x: u8,
            y: u8,
        }

        has_impl_marker::<TestEventC>();
    }

    #[test]
    fn generic_marker_derive() {
        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct GenericEvent<T>(T);
        has_impl_marker::<GenericEvent<u128>>();
        has_impl_marker::<GenericEvent<String>>();

        // This should fail to compile if uncommented, as GenericType does not implement EventRequirements
        //struct GenericType;
        //has_impl_marker::<GenericEvent<GenericType>>();

        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct GenericEvent2<T>(T);
        has_impl_marker::<GenericEvent2<u128>>();
        has_impl_marker::<GenericEvent2<String>>();
    }
}
