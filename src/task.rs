/// A type-erased closure — the unit of work in the thread pool.
///
/// `Task` wraps a `Box<dyn FnOnce() + Send>` so the pool can store
/// heterogeneous closures of different sizes and types in the same queue.
pub struct Task {
    f: Box<dyn FnOnce() + Send + 'static>,
}

impl Task {
    /// Wraps a closure into a type-erased `Task`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pool_of_threads::Task;
    ///
    /// let task = Task::new(|| println!("hello"));
    /// task.run();
    /// ```
    pub fn new(f: impl FnOnce() + Send + 'static) -> Self {
        Self { f: Box::new(f) }
    }

    /// Executes the stored closure, consuming the `Task`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pool_of_threads::Task;
    ///
    /// let mut x = 0;
    /// let task = Task::new(move || x += 1);
    /// task.run();
    /// ```
    pub fn run(self) {
        (self.f)();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn runs_closure() {
        let task = Task::new(|| println!("task executed"));
        task.run();
    }

    #[test]
    fn moves_to_another_thread() {
        let msg = String::from("sent");
        let task = Task::new(move || println!("{msg}"));

        thread::spawn(move || task.run()).join().unwrap();
    }

    #[test]
    fn captures_owned_data() {
        let data = vec![1, 2, 3];
        let task = Task::new(move || {
            assert_eq!(data.len(), 3);
            drop(data);
        });
        task.run();
    }
}
