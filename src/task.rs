pub struct Task {
    f: Box<dyn FnOnce() + Send + 'static>,
}

impl Task {
    pub fn new(f: impl FnOnce() + Send + 'static) -> Self {
        Self { f: Box::new(f) }
    }

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
