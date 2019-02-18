pub trait DriverTask {
    // unsafe fn get_task(&self) -> impl Generator<Yield = (), Return = ()>;
    //
    // should also be here, but rust does not allow it.

    fn has_message(&self) -> bool;
}

pub trait DriverTaskWithState: DriverTask {
    fn is_active(&self) -> bool;
}

pub trait DriverTaskClient {
    fn has_message(&self) -> bool;

    fn reap_message(&self);
}
