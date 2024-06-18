use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
};

type Job = Box<dyn FnOnce() + Send + 'static>;
pub struct ThreadPool {
    job_queue: Arc<JobQueue>,
    pool: Box<[(Arc<ThreadLock>, thread::JoinHandle<()>)]>,
}

impl ThreadPool {
    pub fn new(thread_count: usize) -> Self {
        assert!(thread_count > 0);
        let mut thread_pool_vec = Vec::with_capacity(thread_count);

        let job_queue = Arc::new(JobQueue::new());

        for _ in 0..thread_count {
            let thread_lock = Arc::new(ThreadLock::new());

            let queue_handle = Arc::clone(&job_queue);
            let thread_lock_handle = Arc::clone(&thread_lock);

            let thread_handle = thread::spawn(move || loop {
                // dont do anything unless the job queue threadlock is blocked
                let job = queue_handle.get_job();
                thread_lock_handle.block();
                if let Some(job) = job {
                    job();
                }
                thread_lock_handle.unblock();
            });

            thread_pool_vec.push((thread_lock, thread_handle));
        }

        Self {
            job_queue,
            pool: thread_pool_vec.into_boxed_slice(),
        }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&mut self, job: F) {
        self.job_queue.assign_job(Box::new(job));
    }

    /// blocks the current thread until all the currently queued jobs are finished
    pub fn wait(&self) {
        self.job_queue.wait_for_clear();
        for (lock, _) in self.pool.iter() {
            lock.wait();
        }
    }
}

struct ThreadLock {
    blocking: Mutex<bool>,
    cvar: Condvar,
}

impl ThreadLock {
    pub fn new() -> Self {
        Self {
            blocking: Mutex::new(false),
            cvar: Condvar::new(),
        }
    }

    pub fn block(&self) {
        *self.blocking.lock().unwrap() = true;
    }

    pub fn unblock(&self) {
        *self.blocking.lock().unwrap() = false;
        self.cvar.notify_one();
    }

    pub fn wait(&self) {
        let mut is_blocking = self.blocking.lock().unwrap();

        while *is_blocking {
            is_blocking = self.cvar.wait(is_blocking).unwrap();
        }
    }
}

struct JobQueue {
    queue: Mutex<VecDeque<Job>>,
    is_empty: ThreadLock,
    has_task: ThreadLock,
}

impl JobQueue {
    pub fn new() -> Self {
        let has_task = ThreadLock::new();
        has_task.block();
        Self {
            queue: Mutex::new(VecDeque::new()),
            is_empty: ThreadLock::new(),
            has_task,
        }
    }

    pub fn assign_job(&self, job: Job) {
        self.queue.lock().unwrap().push_back(job);
        self.is_empty.block();
        self.has_task.unblock();
    }

    pub fn get_job(&self) -> Option<Job> {
        self.has_task.wait();
        let mut queue = self.queue.lock().unwrap();
        if let Some(job) = queue.pop_front() {
            self.has_task.unblock();
            Some(job)
        } else {
            self.is_empty.unblock();
            self.has_task.block();
            None
        }
    }

    pub fn wait_for_clear(&self) {
        self.is_empty.wait();
    }
}
