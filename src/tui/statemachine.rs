use std::fmt::Display;

pub enum SmEvent<E> {
    EnterState,
    ExitState,
    Event(E),
}

pub enum SmResult<C, E> {
    EventHandled,
    ChangeState(fn(&mut C, &SmEvent<E>) -> SmResult<C, E>),
    Error,
}

#[derive(Debug)]
pub struct StateMachine<C, E> {
    current_state: fn(&mut C, &SmEvent<E>) -> SmResult<C, E>,
}

impl<C, E> StateMachine<C, E> {
    pub fn new(initial_state: fn(&mut C, &SmEvent<E>) -> SmResult<C, E>) -> StateMachine<C, E> {
        StateMachine{current_state: initial_state}
    }

    pub fn init(&mut self, context: &mut C) {
        self.handle_event(context, &SmEvent::EnterState)
    }

    pub fn handle_event(&mut self, context: &mut C, event: &SmEvent<E>) {
        let result = (self.current_state)(context, event);
        match result {
            SmResult::EventHandled => {},
            SmResult::ChangeState(new_state) => {
                (self.current_state)(context, &SmEvent::ExitState);
                self.current_state = new_state;
                (self.current_state)(context, &SmEvent::EnterState);
            }
            SmResult::Error => panic!("Error handling event")
        }
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

#[cfg(test)]
mod tests {
    use super::SmEvent;
    use super::SmResult;
    use super::StateMachine;

    #[derive(Debug, PartialEq)]
    enum TestState {
        Init,
        Entered,
        Handled,
        Exited
    }

    /* Test object that records the state machine progress for feedback */
    struct TestApp {
        state1: TestState,
        state2: TestState,
        state3: TestState,
        last_value: i32
    }

    /* Test object that implements the state machine functions (states) */
    impl TestApp {
        fn new() -> TestApp {
            TestApp{
                state1: TestState::Init,
                state2: TestState::Init,
                state3: TestState::Init,
                last_value: 0,
            }
        }

        fn state_1(context: &mut TestApp, event: &SmEvent<i32>) -> SmResult<TestApp, i32> {
            match event {
            SmEvent::EnterState => {context.state1 = TestState::Entered; SmResult::EventHandled},
            SmEvent::ExitState => {context.state1 = TestState::Exited; SmResult::EventHandled},
            SmEvent::Event(e) => {
                match e {
                2 => SmResult::ChangeState(TestApp::state_2),
                3 => SmResult::ChangeState(TestApp::state_3),
                _ => {
                    context.last_value = *e;
                    context.state1 = TestState::Handled;
                    SmResult::EventHandled
                }
                }
            },
            }
        }

        fn state_2(context: &mut TestApp, event: &SmEvent<i32>) -> SmResult<TestApp, i32> {
            match event {
            SmEvent::EnterState => {context.state2 = TestState::Entered; SmResult::EventHandled},
            SmEvent::ExitState => {context.state2 = TestState::Exited; SmResult::EventHandled},
            SmEvent::Event(e) => {
                match e {
                1 => SmResult::ChangeState(TestApp::state_1),
                3 => SmResult::ChangeState(TestApp::state_3),
                _ => {
                    context.last_value = *e;
                    context.state1 = TestState::Handled;
                    SmResult::EventHandled
                }
                }
            },
            }
        }

        fn state_3(context: &mut TestApp, event: &SmEvent<i32>) -> SmResult<TestApp, i32> {
            match event {
            SmEvent::EnterState => {context.state3 = TestState::Entered; SmResult::EventHandled},
            SmEvent::ExitState => {context.state3 = TestState::Exited; SmResult::EventHandled},
            SmEvent::Event(e) => {
                match e {
                1 => SmResult::ChangeState(TestApp::state_1),
                2 => SmResult::ChangeState(TestApp::state_2),
                _ => {
                    context.last_value = *e;
                    context.state1 = TestState::Handled;
                    SmResult::EventHandled
                }
                }
            },
            }
        }
    }

    struct TestContext {
        sm: StateMachine<TestApp, i32>,
        app: TestApp,
    }

    impl TestContext {
        fn new() -> TestContext {
            let mut tc = TestContext{sm: StateMachine::new(TestApp::state_1),
                                    app: TestApp::new()};
            tc.sm.init(&mut tc.app);
            tc
        }
    }

    #[test]
    fn test_initial_state_can_be_entered() {
        let context = TestContext::new();

        assert_eq!(context.app.state1, TestState::Entered);
        assert_eq!(context.app.state2, TestState::Init);
        assert_eq!(context.app.state3, TestState::Init);
    }

    #[test]
    fn test_events_are_handled() {
        let mut context = TestContext::new();

        assert_eq!(context.app.state1, TestState::Entered);
        assert_eq!(context.app.last_value, 0);

        context.sm.handle_event(&mut context.app, &SmEvent::Event(42));

        assert_eq!(context.app.state1, TestState::Handled);
        assert_eq!(context.app.last_value, 42);
    }

    #[test]
    fn test_state_can_be_changed() {
        let mut context = TestContext::new();

        context.sm.handle_event(&mut context.app, &SmEvent::Event(2));

        assert_eq!(context.app.state1, TestState::Exited);
        assert_eq!(context.app.state2, TestState::Entered);
        assert_eq!(context.app.state3, TestState::Init);

        context.sm.handle_event(&mut context.app, &SmEvent::Event(3));

        assert_eq!(context.app.state1, TestState::Exited);
        assert_eq!(context.app.state2, TestState::Exited);
        assert_eq!(context.app.state3, TestState::Entered);
    }
}
