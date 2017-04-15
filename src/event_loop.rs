// Copyright 2017 Gilad Naaman
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
// http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! An EPoll-based event loop.
//!
//! Usage and initialization is very similar to EPoll, but flexability is
//! decreased in favour of the general use-case.
//!
//! All registerd files are registerd as EPOLLIN.
//!
//! # Example
//!
//! ```no-run rust
//! // If we want to use different types of files, we must store them
//! // as trait-objects. If that's the situation, we must specify the trait (here, AsRawFd).
//! // The trait must inherit AsRawFd
//! let mut epoll = EventLoop::<AsRawFd>::new().unwrap();
//! 
//! // Register a file-like object onto the epoll.
//! // The last parameter is a user-defined identifier
//! epoll.add(&some_pipe)?;
//! epoll.add(&timer)?;
//! 
//! for e in epoll.wait(Timeout::Milliseconds(500)).unwrap() {
//!     match e.data {
//!         0 => { /* Do something with the pipe  */ },
//!         1 => { /* Do something with the timer */ },
//!         _ => unreachable!()
//!     };
//! }
//! ```

use super::*;

pub struct EventLoop<'a, T: AsRawFd + ?Sized + 'a> {
    epoll: EPoll,
    files: Vec<&'a T>,
    events: Vec<Event>,
}

impl<'a, T: AsRawFd + ?Sized + 'a> EventLoop<'a, T> {
    /// Creates a new event loop
    pub fn new() -> std::io::Result<EventLoop<'a, T>> {
        Ok(EventLoop {
               epoll: EPoll::new()?,
               files: Vec::new(),
               events: Vec::new(),
           })
    }

    /// Registers a file onto the event loop.
    pub fn add(&mut self, file: &'a T) -> io::Result<()> {
        self.epoll.add(file, EPOLLIN, file.as_raw_fd() as u64)?;
        self.files.push(file);

        if self.events.len() < self.files.len() {
            self.events.push(Default::default());
        }

        Ok(())
    }

    /// Removes a file from the event loop.
    pub fn remove(&mut self, file: &'a T) -> io::Result<()> {
        self.epoll.remove(file)?;

        if let Some(index) = self.find_file_index(file.as_raw_fd()) {
            self.files.remove(index);
        }

        Ok(())
    }

    /// Waits for incoming events and returns an iterator over the
    /// files that raised the events.
    pub fn wait(&mut self, timeout: Timeout) -> io::Result<EventLoopIterator<T>> {
        let event_amount = self.epoll.wait(&mut self.events, timeout)?;

        Ok(EventLoopIterator {
               event_loop: self,
               index: 0,
               amount: event_amount,
           })
    }

    /// Returns the index of a file using its descriptor.
    #[inline(always)]
    fn find_file_index(&self, fd: RawFd) -> Option<usize> {
        self.files.iter().position(|i| i.as_raw_fd() == fd)
    }

    /// Returns the index of a file using an event.
    fn find_file_index_by_event(&self, event_index: usize) -> Option<usize> {
        self.find_file_index(self.events[event_index].data as RawFd)
    }
}

/// An iterator over an event loop.
pub struct EventLoopIterator<'a, 'b: 'a, T: AsRawFd + ?Sized + 'b> {
    event_loop: &'a EventLoop<'b, T>,
    index: usize,
    amount: usize,
}

impl<'a, 'b: 'a, T: AsRawFd + ?Sized + 'b> Iterator for EventLoopIterator<'a, 'b, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'b T> {
        if self.index >= self.amount {
            None
        } else {
            let idx = self.index;
            self.index += 1;

            self.event_loop
                .find_file_index_by_event(idx)
                .map(|i| self.event_loop.files[i])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fd(RawFd, u32);

    impl AsRawFd for Fd {
        fn as_raw_fd(&self) -> RawFd {
            self.0
        }
    }

    struct Fd2(RawFd);

    impl AsRawFd for Fd2 {
        fn as_raw_fd(&self) -> RawFd {
            self.0
        }
    }

    #[repr(C)]
    pub struct itimerspec {
        it_interval: libc::timespec,
        it_value: libc::timespec,
    }

    extern "C" {
        fn timerfd_create(clockid: libc::c_int, flags: libc::c_int) -> libc::c_int;
        fn timerfd_settime(fd: RawFd,
                           flags: libc::c_int,
                           new_value: *const itimerspec,
                           old_value: *mut itimerspec)
                           -> libc::c_int;
    }

    #[test]
    fn no_event() {
        let timerfd = unsafe { timerfd_create(libc::CLOCK_MONOTONIC, 0) };
        assert!(timerfd >= 0);
        let timer = Fd(timerfd as RawFd, 0xDEADBEEF);

        let mut epoll = EventLoop::new().unwrap();
        epoll.add(&timer).unwrap();

        let mut times = 0;
        for i in epoll.wait(Timeout::Immediate).unwrap() {
            assert_eq!(i.as_raw_fd(), timerfd);
            times += 1;
        }

        assert_eq!(times, 0);
    }

    #[test]
    fn yes_event() {
        let timeout = itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: 1,
                tv_nsec: 0,
            },
        };

        let timerfd = unsafe { timerfd_create(libc::CLOCK_MONOTONIC, 0) };
        assert!(timerfd >= 0);
        let fd = Fd(timerfd as RawFd, 0xDEADBEEF);
        let fd2 = Fd2(0);

        // Here we're creating a an eventloop that contains trait objects.
        let mut epoll = EventLoop::<AsRawFd>::new().unwrap();
        epoll.add(&fd).unwrap();
        epoll.add(&fd2).unwrap();

        let res = unsafe { timerfd_settime(timerfd, 0, &timeout, std::ptr::null_mut()) };
        assert!(res >= 0);

        let mut times = 0;
        for i in epoll.wait(Timeout::Milliseconds(1000)).unwrap() {
            assert_eq!(i.as_raw_fd(), timerfd); // STDIN should probably not pop up.
            times += 1;
        }

        assert_eq!(times, 1);
    }
}