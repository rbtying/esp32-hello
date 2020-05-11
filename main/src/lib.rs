#![no_std]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

extern crate alloc;

use core::alloc::Layout;
use core::fmt::Write as _;
use core::panic::PanicInfo;

use esp_idf_hal::{gpio, i2c};

use embedded_hal::digital::v2::OutputPin as _;

use ssd1306::{prelude::*, Builder};

pub mod errors;
pub mod freertos_task;
pub mod freertos_units;

#[no_mangle]
pub fn app_main() {
    errors::EspError(unsafe { esp_idf_sys::nvs_flash_init() })
        .into_result()
        .unwrap();

    let oled_fn = move || {
        let oled_i2c_master = unsafe {
            i2c::Master::new(
                i2c::Port::Port0,
                i2c::PinConfig {
                    pin_num: 4,
                    pullup: true,
                },
                i2c::PinConfig {
                    pin_num: 15,
                    pullup: true,
                },
                400_000,
            )
        }
        .unwrap();
        let mut oled_reset = unsafe { gpio::OutputPin::new(16) };
        let mut disp: TerminalMode<_> = Builder::new().connect_i2c(oled_i2c_master).into();
        disp.reset(&mut oled_reset, &mut freertos_task::CurrentTask)
            .unwrap();
        disp.init().unwrap();
        disp.clear().unwrap();
        disp.display_on(true).unwrap();

        let mut n = 0;
        loop {
            n += 1;
            disp.set_position(0, 0).unwrap();
            let _ = writeln!(&mut disp, "loop {}", n);
            freertos_task::CurrentTask::delay(freertos_units::Duration::ms(100));
        }
    };
    let _oled_task_h = freertos_task::Task::new()
        .name("oled_task")
        .stack_size(8192)
        .core_affinity(freertos_task::CpuAffinity::Cpu(freertos_task::Cpu::Pro))
        .start(oled_fn)
        .unwrap();

    let led_blink_fn = move || {
        let mut led_gpio = unsafe { gpio::OutputPin::new(25) };

        let mut n = 0;
        loop {
            n += 1;
            let _ = writeln!(&mut PrintF, "loop {}", n);

            led_gpio.set_high().unwrap();
            freertos_task::CurrentTask::delay(freertos_units::Duration::ms(500));
            led_gpio.set_low().unwrap();
            freertos_task::CurrentTask::delay(freertos_units::Duration::ms(500));
        }
    };

    let _led_blink_h = freertos_task::Task::new()
        .name("led_blink_task")
        .core_affinity(freertos_task::CpuAffinity::Cpu(freertos_task::Cpu::Pro))
        .start(led_blink_fn)
        .unwrap();

    initialize_wifi();
}

struct EventGroupHandle {
    pub event_group_h: core::cell::RefCell<esp_idf_sys::EventGroupHandle_t>,
}
unsafe impl Send for EventGroupHandle {}
unsafe impl Sync for EventGroupHandle {}

const CONNECTED_BIT: esp_idf_sys::UBaseType_t = esp_idf_sys::BIT0;
const ESPTOUCH_DONE_BIT: esp_idf_sys::UBaseType_t = esp_idf_sys::BIT1;

fn initialize_wifi() {
    static S_WIFI_EVENT_GROUP: EventGroupHandle = EventGroupHandle {
        event_group_h: core::cell::RefCell::new(core::ptr::null_mut()),
    };

    unsafe {
        esp_idf_sys::tcpip_adapter_init();
        *S_WIFI_EVENT_GROUP.event_group_h.borrow_mut() = esp_idf_sys::xEventGroupCreate();

        errors::EspError(esp_idf_sys::esp_event_loop_create_default())
            .into_result()
            .unwrap();

        // WIFI_INIT_CONFIG_DEFAULT
        let cfg = esp_idf_sys::wifi_init_config_t {
            event_handler: Some(esp_idf_sys::esp_event_send),
            osi_funcs: &mut esp_idf_sys::g_wifi_osi_funcs,
            wpa_crypto_funcs: esp_idf_sys::g_wifi_default_wpa_crypto_funcs,
            static_rx_buf_num: esp_idf_sys::CONFIG_ESP32_WIFI_STATIC_RX_BUFFER_NUM as i32,
            dynamic_rx_buf_num: esp_idf_sys::CONFIG_ESP32_WIFI_DYNAMIC_RX_BUFFER_NUM as i32,
            tx_buf_type: esp_idf_sys::CONFIG_ESP32_WIFI_TX_BUFFER_TYPE as i32,
            static_tx_buf_num: esp_idf_sys::WIFI_STATIC_TX_BUFFER_NUM as i32,
            dynamic_tx_buf_num: esp_idf_sys::WIFI_DYNAMIC_TX_BUFFER_NUM as i32,
            csi_enable: esp_idf_sys::WIFI_CSI_ENABLED as i32,
            nvs_enable: esp_idf_sys::WIFI_NVS_ENABLED as i32,
            ampdu_rx_enable: esp_idf_sys::WIFI_AMPDU_RX_ENABLED as i32,
            ampdu_tx_enable: esp_idf_sys::WIFI_AMPDU_TX_ENABLED as i32,
            nano_enable: esp_idf_sys::WIFI_NANO_FORMAT_ENABLED as i32,
            tx_ba_win: esp_idf_sys::CONFIG_ESP32_WIFI_TX_BA_WIN as i32,
            rx_ba_win: esp_idf_sys::CONFIG_ESP32_WIFI_RX_BA_WIN as i32,
            wifi_task_core_id: esp_idf_sys::WIFI_TASK_CORE_ID as i32,
            beacon_max_len: esp_idf_sys::WIFI_SOFTAP_BEACON_MAX_LEN as i32,
            mgmt_sbuf_num: esp_idf_sys::WIFI_MGMT_SBUF_NUM as i32,
            feature_caps: esp_idf_sys::g_wifi_feature_caps,
            magic: esp_idf_sys::WIFI_INIT_CONFIG_MAGIC as i32,
        };

        errors::EspError(esp_idf_sys::esp_wifi_init(&cfg))
            .into_result()
            .unwrap();

        errors::EspError(esp_idf_sys::esp_event_handler_register(
            esp_idf_sys::WIFI_EVENT,
            esp_idf_sys::ESP_EVENT_ANY_ID,
            Some(wifi_event_handler),
            core::ptr::null_mut(),
        ))
        .into_result()
        .unwrap();
        errors::EspError(esp_idf_sys::esp_event_handler_register(
            esp_idf_sys::IP_EVENT,
            esp_idf_sys::ip_event_t_IP_EVENT_STA_GOT_IP as i32,
            Some(wifi_event_handler),
            core::ptr::null_mut(),
        ))
        .into_result()
        .unwrap();
        errors::EspError(esp_idf_sys::esp_event_handler_register(
            esp_idf_sys::SC_EVENT,
            esp_idf_sys::ESP_EVENT_ANY_ID,
            Some(wifi_event_handler),
            core::ptr::null_mut(),
        ))
        .into_result()
        .unwrap();

        errors::EspError(esp_idf_sys::esp_wifi_set_mode(
            esp_idf_sys::wifi_mode_t_WIFI_MODE_STA,
        ))
        .into_result()
        .unwrap();
        errors::EspError(esp_idf_sys::esp_wifi_start())
            .into_result()
            .unwrap();
    }

    fn smartconfig_example_task() {
        use esp_idf_sys::{
            esp_smartconfig_set_type, esp_smartconfig_start, esp_smartconfig_stop,
            smartconfig_start_config_t, xEventGroupWaitBits, TickType_t,
        };

        // SC_TYPE_ESPTOUCH
        errors::EspError(unsafe { esp_smartconfig_set_type(0) })
            .into_result()
            .unwrap();

        let cfg = smartconfig_start_config_t { enable_log: false };

        errors::EspError(unsafe { esp_smartconfig_start(&cfg) })
            .into_result()
            .unwrap();

        loop {
            let ux_bits = unsafe {
                xEventGroupWaitBits(
                    *S_WIFI_EVENT_GROUP.event_group_h.borrow(),
                    CONNECTED_BIT | ESPTOUCH_DONE_BIT,
                    1,
                    0,
                    TickType_t::max_value(),
                )
            };

            if ux_bits & CONNECTED_BIT != 0 {
                let _ = writeln!(&mut PrintF, "Wifi connected to AP");
            }
            if ux_bits & ESPTOUCH_DONE_BIT != 0 {
                let _ = writeln!(&mut PrintF, "SmartConfig over");
                unsafe { esp_smartconfig_stop() };
                break;
            }
        }
    }

    extern "C" fn wifi_event_handler(
        _arg: *mut esp_idf_sys::types::c_void,
        event_base: esp_idf_sys::esp_event_base_t,
        event_id: i32,
        event_data: *mut esp_idf_sys::types::c_void,
    ) {
        enum EventBase {
            WifiEvent,
            IpEvent,
            ScEvent,
        }

        let event_base = unsafe {
            if event_base == esp_idf_sys::WIFI_EVENT {
                Some(EventBase::WifiEvent)
            } else if event_base == esp_idf_sys::IP_EVENT {
                Some(EventBase::IpEvent)
            } else if event_base == esp_idf_sys::SC_EVENT {
                Some(EventBase::ScEvent)
            } else {
                None
            }
        };

        match (event_base, event_id as u32) {
            (Some(EventBase::WifiEvent), esp_idf_sys::wifi_event_t_WIFI_EVENT_STA_START) => {
                freertos_task::Task::new()
                    .name("smartconfig_example_task")
                    .stack_size(4096)
                    .core_affinity(freertos_task::CpuAffinity::Cpu(freertos_task::Cpu::Pro))
                    .priority(freertos_task::TaskPriority(3))
                    .start(smartconfig_example_task)
                    .unwrap();
            }
            (Some(EventBase::WifiEvent), esp_idf_sys::wifi_event_t_WIFI_EVENT_STA_DISCONNECTED) => {
                unsafe {
                    esp_idf_sys::xEventGroupClearBits(
                        *S_WIFI_EVENT_GROUP.event_group_h.borrow(),
                        CONNECTED_BIT,
                    )
                };
            }
            (Some(EventBase::IpEvent), esp_idf_sys::ip_event_t_IP_EVENT_STA_GOT_IP) => {
                unsafe {
                    esp_idf_sys::xEventGroupSetBits(
                        *S_WIFI_EVENT_GROUP.event_group_h.borrow(),
                        CONNECTED_BIT,
                    )
                };
            }
            (Some(EventBase::ScEvent), esp_idf_sys::smartconfig_event_t_SC_EVENT_SCAN_DONE) => {
                let _ = writeln!(&mut PrintF, "Scan done");
            }
            (Some(EventBase::ScEvent), esp_idf_sys::smartconfig_event_t_SC_EVENT_FOUND_CHANNEL) => {
                let _ = writeln!(&mut PrintF, "Found channel");
            }
            (Some(EventBase::ScEvent), esp_idf_sys::smartconfig_event_t_SC_EVENT_GOT_SSID_PSWD) => {
                let _ = writeln!(&mut PrintF, "Got SSID and password");
                let evt =
                    unsafe { *(event_data as *mut esp_idf_sys::smartconfig_event_got_ssid_pswd_t) };

                unsafe {
                    let mut wifi_config: esp_idf_sys::wifi_config_t = core::mem::zeroed();
                    wifi_config.sta.ssid.copy_from_slice(&evt.ssid);
                    wifi_config.sta.password.copy_from_slice(&evt.password);
                    wifi_config.sta.bssid_set = evt.bssid_set;
                    if wifi_config.sta.bssid_set {
                        wifi_config.sta.bssid.copy_from_slice(&evt.bssid);
                    }

                    let _ = writeln!(
                        &mut PrintF,
                        "SSID: {:?}",
                        cstr_core::CStr::from_bytes_with_nul_unchecked(&wifi_config.sta.ssid)
                    );
                    let _ = writeln!(
                        &mut PrintF,
                        "Password: {:?}",
                        cstr_core::CStr::from_bytes_with_nul_unchecked(&wifi_config.sta.password)
                    );

                    errors::EspError(esp_idf_sys::esp_wifi_disconnect())
                        .into_result()
                        .unwrap();

                    errors::EspError(esp_idf_sys::esp_wifi_set_config(
                        esp_idf_sys::esp_interface_t_ESP_IF_WIFI_STA,
                        &mut wifi_config as *mut _,
                    ))
                    .into_result()
                    .unwrap();

                    errors::EspError(esp_idf_sys::esp_wifi_connect())
                        .into_result()
                        .unwrap();
                }
            }
            (Some(EventBase::ScEvent), esp_idf_sys::smartconfig_event_t_SC_EVENT_SEND_ACK_DONE) => {
                unsafe {
                    esp_idf_sys::xEventGroupSetBits(
                        *S_WIFI_EVENT_GROUP.event_group_h.borrow(),
                        ESPTOUCH_DONE_BIT,
                    )
                };
            }
            (_, _) => (),
        }
    }
}

struct PrintF;

impl core::fmt::Write for PrintF {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();

        let num_written = unsafe {
            esp_idf_sys::printf(
                b"%.*s\0".as_ptr() as *const _,
                bytes.len(),
                bytes.as_ptr() as *const _,
            )
        };
        if num_written == bytes.len() as i32 {
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}

#[global_allocator]
static ALLOC: esp_idf_alloc::EspIdfAllocator = esp_idf_alloc::EspIdfAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        let _ = writeln!(&mut PrintF, "panic msg: {}", s);
    }
    if let Some(args) = info.message() {
        let _ = writeln!(&mut PrintF, "{}", args);
    }
    if let Some(location) = info.location() {
        let _ = writeln!(
            &mut PrintF,
            "panic location: {}:{}",
            location.file(),
            location.line(),
        );
    }
    unsafe { esp_idf_sys::abort() }
    unreachable!("post-abort")
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    unsafe { esp_idf_sys::abort() }
    unreachable!("post-abort")
}
