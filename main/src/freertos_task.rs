//! Adapted from `freertos_rs`'s `Task` abstraction.

use alloc::boxed::Box;
use cstr_core::{CStr, CString};
use esp_idf_sys::{
    pcTaskGetTaskName, types::c_void, uxTaskGetStackHighWaterMark, vTaskDelay, vTaskDelete,
    xTaskCreatePinnedToCore, xTaskGetCurrentTaskHandle, xTaskGetCurrentTaskHandleForCPU,
    xTaskNotify, xTaskNotifyWait,
};

use crate::errors::{Error, FreeRTOSError};
use crate::freertos_units::DurationTicks;

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum Cpu {
    Pro = 0,
    App = 1,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum CpuAffinity {
    Cpu(Cpu),
    NoAffinity,
}

impl CpuAffinity {
    pub fn as_core_id(self) -> esp_idf_sys::BaseType_t {
        match self {
            CpuAffinity::Cpu(cpu) => cpu as esp_idf_sys::BaseType_t,
            // tskNO_AFFINITY = INT_MAX
            CpuAffinity::NoAffinity => esp_idf_sys::BaseType_t::max_value(),
        }
    }
}

/// Handle for a FreeRTOS task
#[derive(Debug)]
pub struct Task {
    task_handle: esp_idf_sys::TaskHandle_t,
}
unsafe impl Send for Task {}

/// Task's execution priority. Low priority numbers denote low priority tasks.
#[derive(Debug, Copy, Clone)]
pub struct TaskPriority(pub u8);

/// Notification to be sent to a task.
#[derive(Debug, Copy, Clone)]
pub enum TaskNotification {
    /// Send the event, unblock the task, the task's notification value isn't changed.
    NoAction,
    /// Perform a logical or with the task's notification value.
    SetBits(u32),
    /// Increment the task's notification value by one.
    Increment,
    /// Set the task's notification value to this value.
    OverwriteValue(u32),
    /// Try to set the task's notification value to this value. Succeeds
    /// only if the task has no pending notifications. Otherwise, the
    /// notification call will fail.
    SetValue(u32),
}

impl TaskNotification {
    fn to_freertos(&self) -> (u32, esp_idf_sys::eNotifyAction) {
        match *self {
            TaskNotification::NoAction => (0, esp_idf_sys::eNotifyAction_eNoAction),
            TaskNotification::SetBits(v) => (v, esp_idf_sys::eNotifyAction_eSetBits),
            TaskNotification::Increment => (0, esp_idf_sys::eNotifyAction_eIncrement),
            TaskNotification::OverwriteValue(v) => {
                (v, esp_idf_sys::eNotifyAction_eSetValueWithOverwrite)
            }
            TaskNotification::SetValue(v) => {
                (v, esp_idf_sys::eNotifyAction_eSetValueWithoutOverwrite)
            }
        }
    }
}

impl TaskPriority {
    fn to_freertos(&self) -> esp_idf_sys::UBaseType_t {
        self.0 as esp_idf_sys::UBaseType_t
    }
}

/// Helper for spawning a new task. Instantiate with [`Task::new()`].
///
/// [`Task::new()`]: struct.Task.html#method.new
pub struct TaskBuilder<'a> {
    task_name: &'a str,
    task_stack_size: u32,
    task_priority: TaskPriority,
    cpu_affinity: CpuAffinity,
}

impl<'a> TaskBuilder<'a> {
    /// Set the task's name.
    pub fn name<'b>(self, name: &'b str) -> TaskBuilder<'b>
    where
        'a: 'b,
    {
        TaskBuilder {
            task_name: name,
            ..self
        }
    }

    /// Set the stack size, in words.
    pub fn stack_size(self, stack_size: u32) -> Self {
        TaskBuilder {
            task_stack_size: stack_size,
            ..self
        }
    }

    /// Set the task's priority.
    pub fn priority(self, priority: TaskPriority) -> Self {
        TaskBuilder {
            task_priority: priority,
            ..self
        }
    }

    /// Set the task's CPU affinity.
    ///
    /// See https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-guides/freertos-smp.html
    /// for details.
    pub fn core_affinity(self, affinity: CpuAffinity) -> Self {
        TaskBuilder {
            cpu_affinity: affinity,
            ..self
        }
    }

    /// Start a new task that can't return a value.
    pub fn start(self, func: impl FnOnce() -> () + Send + 'static) -> Result<Task, Error> {
        Task::spawn(
            &self.task_name,
            self.task_stack_size,
            self.task_priority,
            func,
            self.cpu_affinity,
        )
    }
}

impl Task {
    /// Prepare a builder object for the new task.
    pub fn new() -> TaskBuilder<'static> {
        TaskBuilder {
            task_name: "generic_rust_task",
            task_stack_size: 2048,
            task_priority: TaskPriority(1),
            cpu_affinity: CpuAffinity::NoAffinity,
        }
    }

    unsafe fn spawn_inner<'a>(
        f: Box<dyn FnOnce()>,
        name: &str,
        stack_size: u32,
        priority: TaskPriority,
        cpu: CpuAffinity,
    ) -> Result<Task, Error> {
        // We need to box `f` again since `Box<dyn FnOnce()>` is a trait object, which has unknown
        // size.
        let f_ptr = Box::into_raw(Box::new(f));

        let task_handle = {
            let name = CString::new(name).map_err(|_| Error::NotSupported)?;
            let mut task_handle: esp_idf_sys::TaskHandle_t = core::mem::zeroed();

            let ret = FreeRTOSError(xTaskCreatePinnedToCore(
                Some(trampoline),
                name.as_ptr(),
                stack_size,
                f_ptr as *const _ as *mut _,
                priority.to_freertos(),
                &mut task_handle,
                cpu.as_core_id(),
            ))
            .into_result();

            match ret {
                Ok(()) => task_handle,
                Err(e) => {
                    // Make sure that we drop `f` correctly before returning an error.
                    let _ = Box::from_raw(f_ptr);
                    return Err(e);
                }
            }
        };

        extern "C" fn trampoline(main: *mut c_void) {
            let boxed_f = unsafe { Box::from_raw(main as *mut Box<dyn FnOnce()>) };
            boxed_f();
            unsafe { vTaskDelete(core::ptr::null_mut()) };
            // FreeRTOS tasks should never return.
            loop {}
        }

        Ok(Task { task_handle })
    }

    fn spawn(
        name: &str,
        stack_size: u32,
        priority: TaskPriority,
        f: impl FnOnce() -> () + Send + 'static,
        cpu: CpuAffinity,
    ) -> Result<Task, Error> {
        unsafe {
            return Task::spawn_inner(Box::new(f), name, stack_size, priority, cpu);
        }
    }

    /// Get the name of the current task.
    pub fn get_name(&self) -> Result<&'_ CStr, ()> {
        unsafe {
            let name_ptr = pcTaskGetTaskName(self.task_handle);
            if name_ptr.is_null() {
                Err(())
            } else {
                Ok(CStr::from_ptr(name_ptr))
            }
        }
    }

    /// Try to find the task of the current execution context.
    pub fn current() -> Result<Task, Error> {
        let task_handle = unsafe { xTaskGetCurrentTaskHandle() };
        if task_handle.is_null() {
            Err(Error::NotFound)
        } else {
            Ok(Task { task_handle })
        }
    }

    /// Try to find the task of the current execution context.
    pub fn current_on_cpu(cpu: Cpu) -> Result<Task, Error> {
        let task_handle =
            unsafe { xTaskGetCurrentTaskHandleForCPU(cpu as esp_idf_sys::BaseType_t) };
        if task_handle.is_null() {
            Err(Error::NotFound)
        } else {
            Ok(Task { task_handle })
        }
    }

    /// Forcibly set the notification value for this task.
    pub fn set_notification_value(&self, val: u32) -> Result<(), Error> {
        self.notify(TaskNotification::OverwriteValue(val))
    }

    /// Notify this task.
    pub fn notify(&self, notification: TaskNotification) -> Result<(), Error> {
        let (u, e) = notification.to_freertos();
        unsafe {
            FreeRTOSError(xTaskNotify(self.task_handle, u, e)).into_result()?;
        }
        Ok(())
    }

    /// Wait for a notification to be posted.
    pub fn wait_for_notification(
        &self,
        clear_bits_enter: u32,
        clear_bits_exit: u32,
        wait_for: impl DurationTicks,
    ) -> Result<u32, Error> {
        let mut val = 0u32;
        let r = unsafe {
            xTaskNotifyWait(
                clear_bits_enter,
                clear_bits_exit,
                &mut val as *mut _,
                wait_for.to_ticks(),
            )
        };
        FreeRTOSError(r).into_result()?;
        Ok(val)
    }

    /// Get the minimum amount of stack that was ever left on this task.
    pub fn get_stack_high_water_mark(&self) -> u32 {
        unsafe { uxTaskGetStackHighWaterMark(self.task_handle) as u32 }
    }
}

/// Helper methods to be performed on the task that is currently executing.
pub struct CurrentTask;
impl CurrentTask {
    /// Delay the execution of the current task.
    pub fn delay(delay: impl DurationTicks) {
        unsafe {
            vTaskDelay(delay.to_ticks());
        }
    }

    /// Get the minimum amount of stack that was ever left on the current task.
    pub fn get_stack_high_water_mark() -> u32 {
        unsafe { uxTaskGetStackHighWaterMark(core::ptr::null_mut()) as u32 }
    }
}
