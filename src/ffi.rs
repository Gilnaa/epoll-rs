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

use libc::c_int;

bitflags! {
    /// Indicates the types of events an epoll can listen to.
    /// All descriptions are taken from the man page of epoll_ctl(2),
    /// except from the undocumented constants, which are documented purely on guesswork.
    #[repr(C)]
    pub flags EventType: u32 {
        /// The associated file is available for read operations.
        const EPOLLIN = 0x001,

        /// The associated file is available for write operations.
        const EPOLLOUT = 0x004,

        /// There is urgent data available for read operations.
        const EPOLLPRI = 0x002,

        /// Error  condition  happened on the associated file descriptor.  
        /// epoll_waitwill always wait for this event; it is not necessary to set it in events.
        const EPOLLERR = 0x008,

        /// Hang up happened on the associated file descriptor.  epoll_wait(2) will always wait for this event; it is not necessary to set
        /// it  in  events.  Note that when reading from a channel such as a pipe or a stream socket, this event merely indicates that the
        /// peer closed its end of the channel.  Subsequent reads from the channel will return 0 (end of file) only after all  outstanding
        /// data in the channel has been consumed.
        const EPOLLHUP = 0x010,
        
        // Stream socket peer closed connection, or shut down writing half of connection.
        // (This flag is especially  useful  for  writing simple code to detect peer shutdown when using Edge Triggered monitoring.)
        const EPOLLRDHUP = 0x2000,

        /// Sets an exclusive wakeup mode for the epoll file descriptor that is being attached to the target file descriptor, fd.  When  a
        /// wakeup event occurs and multiple epoll file descriptors are attached to the same target file using EPOLLEXCLUSIVE, one or more
        /// of the epoll file descriptors will receive an event with epoll_wait(2).  The default in this scenario (when EPOLLEXCLUSIVE  is
        /// not  set)  is  for all epoll file descriptors to receive an event.  EPOLLEXCLUSIVE is thus useful for avoiding thundering herd
        /// problems in certain scenarios.
        /// 
        /// If the same file descriptor is in multiple epoll instances, some with the EPOLLEXCLUSIVE flag, and others without, then events
        /// will  be provided to all epoll instances that did not specify EPOLLEXCLUSIVE, and at least one of the epoll instances that did
        /// specify EPOLLEXCLUSIVE.
        /// 
        /// The following values may be specified in conjunction with EPOLLEXCLUSIVE: EPOLLIN, EPOLLOUT, EPOLLWAKEUP, and EPOLLET.  EPOLL‐
        /// HUP  and  EPOLLERR  can also be specified, but this is not required: as usual, these events are always reported if they occur,
        /// regardless of whether they are specified in events.  Attempts to specify other values in events yield an  error.   EPOLLEXCLU‐
        /// SIVE  may be used only in an EPOLL_CTL_ADD operation; attempts to employ it with EPOLL_CTL_MOD yield an error.  If EPOLLEXCLU‐
        /// SIVE has been set using epoll_ctl(), then a subsequent EPOLL_CTL_MOD on the same epfd, fd pair yields an  error.   A  call  to
        /// epoll_ctl()  that  specifies  EPOLLEXCLUSIVE  in  events and specifies the target file descriptor fd as an epoll instance will
        /// likewise fail.  The error in all of these cases is EINVAL.
        ///
        /// (since Linux 4.5)
        const EPOLLEXCLUSIVE = 1 << 28,

        /// If EPOLLONESHOT and EPOLLET are clear and the process has the CAP_BLOCK_SUSPEND capability, ensure that the  system  does  not
        /// enter  "suspend"  or "hibernate" while this event is pending or being processed.  The event is considered as being "processed"
        /// from the time when it is returned by a call to epoll_wait(2) until the next call to epoll_wait(2) on the  same  epoll(7)  file
        /// descriptor,  the closure of that file descriptor, the removal of the event file descriptor with EPOLL_CTL_DEL, or the clearing
        /// of EPOLLWAKEUP for the event file descriptor with EPOLL_CTL_MOD.
        ///
        /// (since Linux 3.5)
        const EPOLLWAKEUP = 1 << 29,

        /// Sets the one-shot behavior for the  associated  file  descriptor.   This  means  that  after  an  event  is  pulled  out  with
        /// epoll_wait(2)  the  associated file descriptor is internally disabled and no other events will be reported by the epoll inter‐
        /// face.  The user must call epoll_ctl() with EPOLL_CTL_MOD to rearm the file descriptor with a new event mask.
        const EPOLLONESHOT = 1 << 30,

        /// Sets  the Edge Triggered behavior for the associated file descriptor.  The default behavior for epoll is Level Triggered.  
        /// See epoll(7) for more detailed information about Edge and Level Triggered event distribution architectures.
        const EPOLLET = 1 << 31,

        /// Undocumented: Seems to be equviliant to EPOLLIN
        const EPOLLRDNORM = 0x040,

        /// Undocumented: Seems to be equviliant to EPOLLIN, but for OOB data.
        const EPOLLRDBAND = 0x080,

        /// Undocumented: Seems to be equviliant to EPOLLOUT
        const EPOLLWRNORM = 0x100,

        /// Undocumented: Seems to be equviliant to EPOLLOUT, but for OOB data.
        const EPOLLWRBAND = 0x200,

        /// Undocumented: Seems to be unused by anyone (including the kernel).
        const EPOLLMSG = 0x400,
    }
}   

/// This struct is returned by the Kernel to notify of an EPoll event.
/// The data field is the same as supplied by the user on registeration.
/// The events field contains events that occurd in practice.
///
/// This type is marked Copy so that an array could be initialised like so:
/// ```rust
/// let events = [Event::default(); 1312];
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C,packed)]
pub struct Event {
    pub events: EventType,
    pub data: u64
}

impl Default for Event {
    fn default() -> Event {
        Event { events: EPOLLIN, data: 0 }
    }
}

extern {
    pub fn epoll_create(size: c_int) -> c_int;

    pub fn epoll_create1(flags: c_int) -> c_int;

    pub fn epoll_ctl(epfd: c_int,
                        op: c_int,
                        fd: c_int,
                        event: *mut Event) -> c_int;

    pub fn epoll_wait(epfd: c_int,
                        events: *mut Event,
                        maxevents: c_int,
                        timeout: c_int) -> c_int;
}