#[cfg(test)]
mod tests {
    use al_core::EventMarker;
    use al_derive::EventMarker;

    /// Helper function to ensure a type implements EventMarker
    fn has_impl_marker<T: EventMarker>() {}

    /// Test deriving EventMarker for simple structs
    #[test]
    fn event_marker_derive() {
        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct TestEventA;

        has_impl_marker::<TestEventA>();

        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct TestEventB(String, Vec<u128>);

        has_impl_marker::<TestEventB>();

        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct TestEventC {
            x: u8,
            y: u8
        }

        has_impl_marker::<TestEventC>();
    }

    /*#[test]
    fn generic_marker_derive() {
        #[derive(EventMarker)]
        struct GenericEvent<T>(T);

        
        #[derive(Clone, Default, PartialEq, Hash, Debug)]
        struct GenericEvent2<T>(T);
        impl<T> EventMarker for GenericEvent2<T> {}
        has_impl_marker::<GenericEvent2<u128>>();

        
        #[derive(Clone, Default, PartialEq, Hash, Debug)]
        struct NonGenericEvent;
        impl<> EventMarker for NonGenericEvent<> {}
        has_impl_marker::<NonGenericEvent>();

        has_impl_marker::<GenericEvent<u128>>();
        has_impl_marker::<GenericEvent<String>>();
    }*/
}
