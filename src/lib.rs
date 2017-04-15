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

//! A sufficiently safe wrapper around Linux's EPoll interface.
//!
//! Objects that are "file-like" (i.e. implement AsRawFd) can be registerd
//! on the epoll with a certain event mask.The user then can wait for events
//! from any of the registerd files.
//!
//! The registerd files are identified by used supplied data.
//! The file-descriptors can be used for this.
//!
//! # Example
//! 
//! ```no-run
//! let mut epoll = EPoll::new();
//! 
//! // Register a file-like object onto the epoll.
//! // The last parameter is a user-defined identifier
//! epoll.add(&some_pipe, EPOLLIN, 0)?;
//! epoll.add(&timer, EPOLLIN, 1)?;
//! 
//! let mut events = [Event::default(); 2];
//! let event_count = epoll.wait(&mut events, Timeout::Milliseconds(500))?;
//! for e in &events[..event_count] {
//!     match e.data {
//!         0 => { /* Do something with the socket */ },
//!         1 => { /* Do something with the timer  */ },
//!         _ => unreachable!()
//!     };
//! }
//! ```

#[macro_use] extern crate bitflags;
extern crate libc;

use std::io::{self, Error};
use std::os::unix::io::{RawFd, AsRawFd};

mod ffi;
pub use ffi::*;

/// An object used to poll for many events at once.
pub struct EPoll {
    fd: RawFd
}

impl EPoll {
    /// Creates a new EPoll object.
    pub fn new() -> io::Result<Self> {
        let fd = unsafe {
            ffi::epoll_create1(0)

        };

        if fd < 0 {
            Err(Error::last_os_error())            
        }
        else {
            Ok(EPoll { fd: fd })
        }
    }

    /// Adds a new file-like-object onto the epoll.
    ///
    /// The data parameter is a user-defined identification of the object;
    /// for example, it can be an index to an array, the file-descriptor itself, etc.
    pub fn add<T: AsRawFd + ?Sized>(&mut self, file: &T, events: EventType, data: u64) -> io::Result<()> {
        let mut event = Event { events: events, data: data };
        
        let rc = unsafe { 
            ffi::epoll_ctl(self.fd, 
                            libc::EPOLL_CTL_ADD, 
                            file.as_raw_fd(), 
                            &mut event) 
        };

        if rc < 0 {
            Err(Error::last_os_error())            
        }
        else {
            Ok(())
        }
    }

    /// Removes an existing file-like-object from the epoll.
    pub fn remove<T: AsRawFd + ?Sized>(&mut self, file: &T) -> io::Result<()> {
        // This syscall doesn't actually use the "event" pointer, but earlier kernel versions
        // required it to be non-null.
        let mut event = Event::default();
        
        let rc = unsafe { 
            ffi::epoll_ctl(self.fd, 
                            libc::EPOLL_CTL_DEL, 
                            file.as_raw_fd(),
                            &mut event) 
        };

        if rc < 0 {
            Err(Error::last_os_error())            
        }
        else {
            Ok(())
        }
    }

    /// Modifies the event mask and the associated data of a registered file.
    pub fn modify<T: AsRawFd + ?Sized>(&mut self, file: &T, events: EventType, data: u64) -> io::Result<()> {
        let mut event = Event { events: events, data: data };
        
        let rc = unsafe { 
            ffi::epoll_ctl(self.fd, 
                            libc::EPOLL_CTL_MOD, 
                            file.as_raw_fd(), 
                            &mut event) 
        };

        if rc < 0 {
            Err(Error::last_os_error())            
        }
        else {
            Ok(())
        }
    }

    /// Waits for an event.
    /// 
    /// `events` is an output parameter, which indicates the amount of events the user
    /// is currently able to accept.
    /// The return value is the amount that are ready to be processed, and is in the range 0...events.len().
    ///
    /// # Example
    /// ```no-run
    /// let mut events = [Event::default(); 3];
    /// let interfaces = e.wait(&mut events, Timeout::Indefinite)?;
    /// for e in &events[..interfaces] {
    ///     // ...
    /// }
    /// ```
    pub fn wait(&self, events: &mut [Event], timeout: Timeout) -> io::Result<usize> {
        let timeout = match timeout {
            Timeout::Indefinite => -1,
            Timeout::Immediate => 0,
            Timeout::Milliseconds(amount) => {
                if amount >= std::i32::MAX as usize {
                    std::i32::MAX
                }
                else {
                    amount as i32
                }
            }
        };

        let rc = unsafe {
            ffi::epoll_wait(self.fd, 
                             events.as_mut_ptr(),
                             events.len() as libc::c_int,
                             timeout)
        };

        if rc < 0 {
            Err(Error::last_os_error())
        }
        else {
            Ok(rc as usize)
        }
    }
}

impl AsRawFd for EPoll {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for EPoll {
    fn drop (&mut self) {
        unsafe { libc::close(self.fd as libc::c_int); }

        // Poison the file descriptor.
        self.fd = -1;
    }
}

/// Describes an EPoll wait timeout.
#[derive(Clone, Copy, Debug)]
pub enum Timeout {
    /// The EPoll will wait indefinitely, and will return only when an event is ready, or on error.
    Indefinite,
    
    /// The wait operation will return immediately, even if no events are ready.
    Immediate,

    /// The wait operation will wait `usize` milliseconds for new events before giving up.
    ///
    /// # Notes
    /// This variant is of type `usize`, but is actually capped to std::i32::MAX due to API
    /// restrictions.
    Milliseconds(usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fd(RawFd);

    impl AsRawFd for Fd {
        fn as_raw_fd(&self) -> RawFd { self.0 }
    }
    
    #[repr(C)]
    pub struct itimerspec {
        it_interval: libc::timespec,
        it_value: libc::timespec,
    }

    extern "C" {
        fn timerfd_create(clockid: libc::c_int, flags: libc::c_int) -> libc::c_int;
        fn timerfd_settime(fd: RawFd, flags: libc::c_int,
                        new_value: *const itimerspec, old_value: *mut itimerspec) -> libc::c_int;
    }

    #[test]
    fn no_event() {
        let mut epoll = EPoll::new().unwrap();
        
        let timerfd = unsafe { timerfd_create(libc::CLOCK_MONOTONIC, 0) };
        assert!(timerfd >= 0);
        let timerfd = Fd(timerfd as RawFd);
        
        epoll.add(&timerfd, EPOLLIN, timerfd.as_raw_fd() as u64).unwrap();

        let mut events = [Event::default(); 1];
        
        let res = epoll.wait(&mut events, Timeout::Immediate);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 0);
    }

    #[test]
    fn yes_event() {
        let mut epoll = EPoll::new().unwrap();
        
        let timerfd = unsafe { timerfd_create(libc::CLOCK_MONOTONIC, 0) };
        assert!(timerfd >= 0);
        let timerfd = Fd(timerfd as RawFd);
        
        epoll.add(&timerfd, EPOLLIN, timerfd.as_raw_fd() as u64).unwrap();

        let timeout = itimerspec { 
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0
            },
            it_value: libc::timespec {
                tv_sec: 1,
                tv_nsec: 0
            }
        };
        let res = unsafe { timerfd_settime(timerfd.0, 0, &timeout, std::ptr::null_mut()) };
        assert!(res >= 0);

        let mut events = [Event::default(); 1];

        let res = epoll.wait(&mut events, Timeout::Milliseconds(1000));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1);
    }
}