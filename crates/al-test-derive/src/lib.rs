#[cfg(test)]
mod tests {
    use al_core::EventMarker;
    use al_derive::EventMarker;

    fn has_impl_marker<T: EventMarker>() {}

    #[test]
    fn event_marker_derive() {
        #[derive(Clone, Default, PartialEq, Hash, Debug, EventMarker)]
        struct TestEvent;

        has_impl_marker::<TestEvent>();
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
