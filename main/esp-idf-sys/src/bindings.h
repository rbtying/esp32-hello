#include <driver/gpio.h>
#include <driver/i2c.h>
#include <driver/uart.h>
#include <esp_event.h>
#include <esp_log.h>
#include <esp_smartconfig.h>
#include <esp_system.h>
#include <esp_wifi.h>
#include <esp_wpa2.h>
#include <freertos/FreeRTOS.h>
#include <freertos/event_groups.h>
#include <freertos/task.h>
#include <nvs_flash.h>
#include <tcpip_adapter.h>
#include "sdkconfig.h"
