#include "driver/gpio.h"
#include "driver/i2c.h"
#include "esp_log.h"
#include "esp_now.h"
#include "esp_rom_sys.h" // For esp_rom_delay_us
#include "esp_timer.h"
#include "esp_wifi.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"
#include "freertos/task.h"
#include "nvs_flash.h"
#include <stdio.h>
#include <string.h>

#define TAG "FORMULA_KID_CONTROLLER"

// I2C defines for LED Matrix
#define I2C_NUM_0 I2C_NUM_0
#define I2C_SDA_GPIO_0 GPIO_NUM_21
#define I2C_SCL_GPIO_0 GPIO_NUM_22
#define HT16K33_ADDR 0x70

// Joystick defines
#define JS1_TRIG_GPIO GPIO_NUM_26
#define JS1_CAP_GPIO GPIO_NUM_32
#define JS2_TRIG_GPIO GPIO_NUM_27
#define JS2_CAP_GPIO GPIO_NUM_33

#define R_SERIE 1000.0f
#define RC_FACTOR_5V 9.788075945f
#define CAP_TIMEOUT_US 500000 // 500ms
#define DISCHARGE_MS 10

#define JS1_DEAD_ZONE 10 // JS1 rests near 0, small zone ok
#define JS2_DEAD_ZONE 20 // JS2 rests ~25 at idle, needs larger zone

// ESP-NOW defines
#define ESPNOW_CHANNEL 1
static uint8_t s_broadcast_mac[] = {0x30, 0xae, 0xa4, 0xf0, 0x3f, 0x8c};

// Queue for ISR to send data to task
typedef struct {
  int gpio_num;
  int64_t duration;
} rc_timing_event_t;

static QueueHandle_t s_rc_timing_queue;

// ISR for RC timing
static IRAM_ATTR void rc_timing_isr_handler(void *arg) {
  int gpio_num = (int)arg;
  int64_t stop_time = esp_timer_get_time();
  rc_timing_event_t event = {.gpio_num = gpio_num, .duration = stop_time};
  xQueueSendFromISR(s_rc_timing_queue, &event, NULL);
}

// LED Matrix — Letter characters (Corrected mapping for 180° rotated display)
// - cols[0] is physical Left, cols[15] is physical Right
// - Bit 7 is physical Top, Bit 0 is physical Bottom
static const uint8_t img_up[16] = {0x00, 0x00, 0xFF, 0xFF, 0x01, 0x01,
                                   0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
                                   0xFF, 0xFF, 0x00, 0x00}; // U
static const uint8_t img_down[16] = {0x00, 0x00, 0x00, 0xFF, 0xFF, 0x81,
                                     0x81, 0x81, 0x81, 0x81, 0x81, 0x7E,
                                     0x3C, 0x00, 0x00, 0x00}; // D
static const uint8_t img_left[16] = {0x00, 0x00, 0x00, 0xFF, 0xFF, 0x01,
                                     0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
                                     0x01, 0x00, 0x00, 0x00}; // L
static const uint8_t img_right[16] = {0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF,
                                      0x90, 0x90, 0x98, 0x94, 0x62, 0x01,
                                      0x00, 0x00, 0x00, 0x00}; // R
static const uint8_t img_stop[16] = {0x00, 0x00, 0x18, 0x18, 0x18, 0x18,
                                     0x00, 0x00, 0x00, 0x00, 0x18, 0x18,
                                     0x18, 0x18, 0x00, 0x00}; // --

static void i2c_init_bus0(void) {
  i2c_config_t conf = {
      .mode = I2C_MODE_MASTER,
      .sda_io_num = I2C_SDA_GPIO_0,
      .scl_io_num = I2C_SCL_GPIO_0,
      .sda_pullup_en = GPIO_PULLUP_ENABLE,
      .scl_pullup_en = GPIO_PULLUP_ENABLE,
      .master.clk_speed = 100000,
  };
  ESP_ERROR_CHECK(i2c_param_config(I2C_NUM_0, &conf));
  ESP_ERROR_CHECK(i2c_driver_install(I2C_NUM_0, conf.mode, 0, 0, 0));
  ESP_LOGI(TAG, "I2C Bus 0 initialized (SDA:%d, SCL:%d)", I2C_SDA_GPIO_0,
           I2C_SCL_GPIO_0);
}

static void matrix_init(void) {
  uint8_t cmd_display_on[] = {0x81}; // Display ON
  uint8_t cmd_osc_on[] = {0x21};     // Oscillator ON
  uint8_t cmd_brightness[] = {0xEF}; // Brightness max

  i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, cmd_osc_on,
                             sizeof(cmd_osc_on), pdMS_TO_TICKS(100));
  i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, cmd_display_on,
                             sizeof(cmd_display_on), pdMS_TO_TICKS(100));
  i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, cmd_brightness,
                             sizeof(cmd_brightness), pdMS_TO_TICKS(100));
  ESP_LOGI(TAG, "LED Matrix initialized.");
}

static void matrix_draw(const uint8_t cols[16]) {
  uint8_t buf[17] = {0};
  buf[0] = 0x00; // register pointer
  for (int c = 0; c < 8; c++) {
    buf[1 + (c * 2)] = cols[c];     // Left screen col  ← right half of array
    buf[2 + (c * 2)] = cols[c + 8]; // Right screen col ← left half of array
  }
  i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, buf, sizeof(buf),
                             pdMS_TO_TICKS(100));
}

static void wifi_init(void) {
  ESP_ERROR_CHECK(nvs_flash_init());
  ESP_ERROR_CHECK(esp_netif_init());
  ESP_ERROR_CHECK(esp_event_loop_create_default());
  wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
  ESP_ERROR_CHECK(esp_wifi_init(&cfg));
  ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
  ESP_ERROR_CHECK(esp_wifi_start());
  ESP_ERROR_CHECK(esp_wifi_set_channel(ESPNOW_CHANNEL, WIFI_SECOND_CHAN_NONE));
  ESP_LOGI(TAG, "Wi-Fi initialized for ESP-NOW.");
}

static void espnow_send_cb(const wifi_tx_info_t *tx_info,
                           esp_now_send_status_t status) {
  (void)tx_info; // Suppress unused parameter warning
  if (status == ESP_NOW_SEND_SUCCESS) {
    // ESP_LOGI(TAG, "ESP-NOW send success"); // Don't spam logs
  } else {
    ESP_LOGW(TAG, "ESP-NOW send failed");
  }
}

static void espnow_init(void) {
  ESP_ERROR_CHECK(esp_now_init());
  ESP_ERROR_CHECK(esp_now_register_send_cb(espnow_send_cb));
  esp_now_peer_info_t peer_info = {0};
  memcpy(peer_info.peer_addr, s_broadcast_mac, 6);
  peer_info.channel = ESPNOW_CHANNEL;
  peer_info.ifidx = WIFI_IF_STA;
  peer_info.encrypt = false;
  ESP_ERROR_CHECK(esp_now_add_peer(&peer_info));
  ESP_LOGI(TAG, "ESP-NOW initialized.");
}

// Function to read RC timing for a joystick axis
static int read_rc_timing(gpio_num_t trig_gpio, gpio_num_t cap_gpio,
                          int64_t *out_start_time) {
  // Discharge capacitor
  gpio_intr_disable(cap_gpio);
  gpio_set_level(trig_gpio, 1);
  esp_rom_delay_us(DISCHARGE_MS * 1000); // Convert ms to us

  // Start charging and measure
  gpio_set_level(trig_gpio, 0);
  *out_start_time = esp_timer_get_time();
  gpio_intr_enable(cap_gpio);

  return ESP_OK;
}

static int calculate_joystick_position(int64_t duration, int release,
                                       int min_cal, int max_cal) {
  // resistance = (stop_ts - start_ts) * 9.788075945 - 1000
  // raw_pos = (int)(resistance * 200.0 / 10000.0) - 100
  float resistance = (float)duration * RC_FACTOR_5V - R_SERIE;
  int raw_pos = (int)(resistance * 200.0f / 10000.0f) - 100;

  int pos = raw_pos - release;

  if (pos < 0) {
    pos = (int)((float)pos * 100.0f / (float)abs(min_cal - release));
  } else {
    pos = (int)((float)pos * 100.0f / (float)abs(max_cal - release));
  }

  // Clamp pos to -100..100
  if (pos > 100)
    pos = 100;
  if (pos < -100)
    pos = -100;

  return pos;
}

void app_main(void) {
  ESP_LOGI(TAG, "Starting Formula Kid Controller...");

  // Initialize I2C and LED Matrix
  i2c_init_bus0();
  matrix_init();
  matrix_draw(img_stop); // Initial display

  // Initialize Wi-Fi and ESP-NOW
  wifi_init();
  espnow_init();

  // Configure GPIOs for joysticks
  gpio_config_t io_conf = {0};
  io_conf.intr_type = GPIO_INTR_POSEDGE; // Interrupt on rising edge
  io_conf.mode = GPIO_MODE_INPUT;
  io_conf.pull_up_en = GPIO_PULLUP_DISABLE;
  io_conf.pull_down_en = GPIO_PULLDOWN_DISABLE;
  io_conf.pin_bit_mask = (1ULL << JS1_CAP_GPIO) | (1ULL << JS2_CAP_GPIO);
  gpio_config(&io_conf);

  io_conf.intr_type = GPIO_INTR_DISABLE;
  io_conf.mode = GPIO_MODE_OUTPUT;
  io_conf.pin_bit_mask = (1ULL << JS1_TRIG_GPIO) | (1ULL << JS2_TRIG_GPIO);
  gpio_config(&io_conf);

  // Install ISR service
  gpio_install_isr_service(0);
  gpio_isr_handler_add(JS1_CAP_GPIO, rc_timing_isr_handler,
                       (void *)JS1_CAP_GPIO);
  gpio_isr_handler_add(JS2_CAP_GPIO, rc_timing_isr_handler,
                       (void *)JS2_CAP_GPIO);

  s_rc_timing_queue = xQueueCreate(10, sizeof(rc_timing_event_t));

  int js1_pos = 0;
  int js2_pos = 0;
  int prev_espnow_value = 999;
  const uint8_t *prev_matrix_pattern = img_stop;

  int64_t js1_start_time = 0;
  int64_t js2_start_time = 0;

  while (1) {
    rc_timing_event_t event;
    int64_t current_time = esp_timer_get_time();

    // --- Measure JS1 ---
    // Flush stale events from previous cycle
    while (xQueueReceive(s_rc_timing_queue, &event, 0) == pdTRUE) {
    }
    read_rc_timing(JS1_TRIG_GPIO, JS1_CAP_GPIO, &js1_start_time);
    vTaskDelay(pdMS_TO_TICKS(5)); // Wait for cap to charge

    current_time = esp_timer_get_time();
    if (xQueueReceive(s_rc_timing_queue, &event,
                      pdMS_TO_TICKS(CAP_TIMEOUT_US / 1000)) == pdTRUE &&
        event.gpio_num == JS1_CAP_GPIO) {
      js1_pos = calculate_joystick_position(event.duration - js1_start_time, -3,
                                            -100, 89);
    } else if ((current_time - js1_start_time) > CAP_TIMEOUT_US) {
      ESP_LOGW(TAG, "JS1 Timeout, using last known position.");
    }

    // --- Measure JS2 ---
    // Flush stale events (JS1 ISR might have fired again)
    while (xQueueReceive(s_rc_timing_queue, &event, 0) == pdTRUE) {
    }
    read_rc_timing(JS2_TRIG_GPIO, JS2_CAP_GPIO, &js2_start_time);
    vTaskDelay(pdMS_TO_TICKS(5)); // Wait for cap to charge

    current_time = esp_timer_get_time();
    if (xQueueReceive(s_rc_timing_queue, &event,
                      pdMS_TO_TICKS(CAP_TIMEOUT_US / 1000)) == pdTRUE &&
        event.gpio_num == JS2_CAP_GPIO) {
      js2_pos = calculate_joystick_position(event.duration - js2_start_time, -3,
                                            -100, 90);
    } else if ((current_time - js2_start_time) > CAP_TIMEOUT_US) {
      ESP_LOGW(TAG, "JS2 Timeout, using last known position.");
    }

    int32_t espnow_value = 999;
    const uint8_t *current_matrix_pattern = img_stop;
    const char *direction_label = "STOP  [--]";

    if (js1_pos >= JS1_DEAD_ZONE) { // Forward
      espnow_value = js1_pos;
      current_matrix_pattern = img_up;
      direction_label = "FORWARD [U]";
    } else if (js1_pos <= -JS1_DEAD_ZONE) { // Backward
      espnow_value = js1_pos;
      current_matrix_pattern = img_down;
      direction_label = "BACKWARD[D]";
    } else if (js2_pos >= JS2_DEAD_ZONE) { // Right
      espnow_value = js2_pos + 400;
      current_matrix_pattern = img_right;
      direction_label = "RIGHT   [R]";
    } else if (js2_pos <= -JS2_DEAD_ZONE) { // Left
      espnow_value = js2_pos + 400;
      current_matrix_pattern = img_left;
      direction_label = "LEFT    [L]";
    } else { // Stop
      espnow_value = 999;
      current_matrix_pattern = img_stop;
      direction_label = "STOP    [-]";
    }

    // จับ direction_changed ก่อน update prev_matrix_pattern
    // (หลัง update แล้ว current == prev เสมอ ทำให้ detect ไม่ได้)
    int direction_changed = (current_matrix_pattern != prev_matrix_pattern);

    // Update LED Matrix only if pattern changes
    if (direction_changed) {
      matrix_draw(current_matrix_pattern);
      prev_matrix_pattern = current_matrix_pattern;
    }

    // ส่งเฉพาะตอนทิศทางเปลี่ยน หรือตอนกำลังเคลื่อน (ไม่ใช่ STOP) และค่าเปลี่ยนเกิน 5
    int value_delta = (int)espnow_value - (int)prev_espnow_value;
    if (value_delta < 0)
      value_delta = -value_delta; // abs manually (avoid macro side-effects)
    int moving = (current_matrix_pattern != img_stop);
    if (direction_changed || (moving && value_delta > 5)) {
      esp_err_t ret = esp_now_send(s_broadcast_mac, (uint8_t *)&espnow_value,
                                   sizeof(espnow_value));
      if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Error sending ESP-NOW data: %s", esp_err_to_name(ret));
      }
      ESP_LOGI(TAG, "📡 Sent → Value: %4ld | Dir: %s", (long)espnow_value,
               direction_label);
      prev_espnow_value = espnow_value;
    }

    // Debug: แสดงค่า raw joystick ทุก loop เพื่อตรวจสอบ calibration
    ESP_LOGI(TAG, "JS1_raw=%4d JS2_raw=%4d | threshold±%d", js1_pos, js2_pos,
             JS2_DEAD_ZONE);

    vTaskDelay(pdMS_TO_TICKS(50));
  }
}
