/**
 * @file kidbright_full_system_demo.c
 * @brief Master Template: KidBright32iA Full System Integration (Inputs & Outputs)
 * @note ESP-IDF v5.x compliant.
 *
 * การสาธิตนี้รวมกฎการใช้งานทั้งหมดจาก kidbright32iA.md:
 * 1. ใช้ FreeRTOS Queue และ Task เพื่อป้องกัน Watchdog Reset
 * 2. หลีกเลี่ยง Hardware Conflicts (ใช้ SERVO2 ขา 17 เนื่องจากมี SW1 ขา 16)
 * 3. หลีกเลี่ยงใช้งาน GPIO4 (BT LED) เนื่องจากใช้ LM73 (I2C_NUM_1)
 * 4. การจัดการ I2C 2 บัสแยกกันโดยเด็ดขาด 
 * 5. จัดการ LDR ด้วย ADC Oneshot API แบบใหม่ของ ESP-IDF v5.x
 */

#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/queue.h"
#include "driver/gpio.h"
#include "driver/i2c.h"
#include "driver/ledc.h"
#include "esp_adc/adc_oneshot.h"
#include "esp_log.h"
#include "esp_intr_alloc.h"

static const char *TAG = "KB32_DEMO";

// ==========================================
// 1. PIN CONFIGURATIONS & DEFINITIONS
// ==========================================
// LED Matrix (I2C_NUM_0) & Accelerometer
#define I2C_MASTER_NUM    I2C_NUM_0
#define I2C_MASTER_SDA_IO GPIO_NUM_21
#define I2C_MASTER_SCL_IO GPIO_NUM_22
#define HT16K33_ADDR      0x70

// Temperature Sensor LM73 (I2C_NUM_1)
#define I2C_TEMP_NUM      I2C_NUM_1
#define I2C_TEMP_SDA_IO   GPIO_NUM_4
#define I2C_TEMP_SCL_IO   GPIO_NUM_5
#define LM73_ADDR         0x4D

// LDR (Light Sensor)
#define LDR_ADC_UNIT      ADC_UNIT_1
#define LDR_ADC_CHAN      ADC_CHANNEL_0 // GPIO36

// Buttons (Active LOW)
#define BTN_SW1_GPIO      GPIO_NUM_16
#define BTN_SW2_GPIO      GPIO_NUM_14

// Buzzer
#define BUZZER_GPIO       GPIO_NUM_13
#define BUZZER_LEDC_TIMER LEDC_TIMER_0
#define BUZZER_LEDC_CHAN  LEDC_CHANNEL_0

// Relay (JST OUT1)
#define RELAY_OUT1_GPIO   GPIO_NUM_26

// Servo (SERVO2 - To avoid SW1 conflict)
#define SERVO_GPIO        GPIO_NUM_17
#define SERVO_LEDC_TIMER  LEDC_TIMER_1
#define SERVO_LEDC_CHAN   LEDC_CHANNEL_1
#define SERVO_MIN_TICKS   1638 // ~500us for 0 degree
#define SERVO_MAX_TICKS   8192 // ~2500us for 180 degree

#define ESP_INTR_FLAG_DEFAULT 0

// ==========================================
// 2. GLOBAL HANDLES & QUEUES
// ==========================================
static adc_oneshot_unit_handle_t ldr_adc_handle;
static QueueHandle_t button_evt_queue = NULL;

// Matrix Patterns (Row-Major)
const uint16_t PATTERN_SMILEY[8] = { 0x0000, 0x0C30, 0x0C30, 0x0000, 0x0000, 0x1008, 0x07E0, 0x0000 };
const uint16_t PATTERN_SQUINT[8] = { 0x0000, 0x1E78, 0x0000, 0x0000, 0x0000, 0x1008, 0x0FF0, 0x0000 };

// ==========================================
// 3. HARDWARE INIT & HELPER FUNCTIONS
// ==========================================

// --- LED Matrix ---
void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16]) {
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1 << (15 - col))) {
                out_cols[col] |= (1 << (7 - row)); // Y-axis inversion requirement
            }
        }
    }
}

void matrix_draw(const uint16_t pattern[8]) {
    uint8_t cols[16];
    rows_to_columns_16x8(pattern, cols);
    
    uint8_t buf[17] = {0};
    buf[0] = 0x00; // RAM start address
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];
        buf[2 + (c * 2)] = cols[c + 8];
    }
    i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, buf, sizeof(buf), pdMS_TO_TICKS(100));
}

void matrix_init(void) {
    i2c_config_t conf = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C_MASTER_SDA_IO,
        .scl_io_num = I2C_MASTER_SCL_IO,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = 100000,
    };
    i2c_param_config(I2C_MASTER_NUM, &conf);
    i2c_driver_install(I2C_MASTER_NUM, conf.mode, 0, 0, 0);

    uint8_t cmd;
    cmd = 0x21; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100)); // Osc ON
    cmd = 0x81; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100)); // Display ON
    cmd = 0xEF; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100)); // Max Brightness
    
    matrix_draw(PATTERN_SMILEY);
}

// --- LM73 Temperature ---
void temp_sensor_init(void) {
    i2c_config_t conf = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C_TEMP_SDA_IO,
        .scl_io_num = I2C_TEMP_SCL_IO,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = 100000,
    };
    i2c_param_config(I2C_TEMP_NUM, &conf);
    i2c_driver_install(I2C_TEMP_NUM, conf.mode, 0, 0, 0);
}

float read_lm73(void) {
    uint8_t raw[2];
    uint8_t reg = 0x00;
    esp_err_t ret = i2c_master_write_read_device(I2C_TEMP_NUM, LM73_ADDR, &reg, 1, raw, 2, pdMS_TO_TICKS(100));
    if (ret == ESP_OK) {
        int16_t temp = (int16_t)((raw[0] << 8) | raw[1]);
        return (float)(temp >> 5) / 32.0f; // Default 11-bit conversion
    }
    return -999.0f;
}

// --- LDR ADC (v5.x API) ---
void ldr_init(void) {
    adc_oneshot_unit_init_cfg_t init_config = { .unit_id = LDR_ADC_UNIT };
    ESP_ERROR_CHECK(adc_oneshot_new_unit(&init_config, &ldr_adc_handle));

    adc_oneshot_chan_cfg_t chan_config = {
        .atten    = ADC_ATTEN_DB_12,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    ESP_ERROR_CHECK(adc_oneshot_config_channel(ldr_adc_handle, LDR_ADC_CHAN, &chan_config));
}

int read_ldr(void) {
    int raw = 0;
    adc_oneshot_read(ldr_adc_handle, LDR_ADC_CHAN, &raw);
    return raw; // 0 = bright, 4095 = dark
}

// --- Buzzer PWM ---
void buzzer_init(void) {
    ledc_timer_config_t ledc_timer = {
        .speed_mode       = LEDC_LOW_SPEED_MODE,
        .timer_num        = BUZZER_LEDC_TIMER,
        .duty_resolution  = LEDC_TIMER_10_BIT,
        .freq_hz          = 1000,
        .clk_cfg          = LEDC_AUTO_CLK
    };
    ledc_timer_config(&ledc_timer);

    ledc_channel_config_t ledc_channel = {
        .speed_mode     = LEDC_LOW_SPEED_MODE,
        .channel        = BUZZER_LEDC_CHAN,
        .timer_sel      = BUZZER_LEDC_TIMER,
        .intr_type      = LEDC_INTR_DISABLE,
        .gpio_num       = BUZZER_GPIO,
        .duty           = 0,
        .hpoint         = 0
    };
    ledc_channel_config(&ledc_channel);
}

void play_tone(uint32_t freq, uint32_t duration_ms) {
    if (freq > 0) {
        ledc_set_freq(LEDC_LOW_SPEED_MODE, BUZZER_LEDC_TIMER, freq);
        ledc_set_duty(LEDC_LOW_SPEED_MODE, BUZZER_LEDC_CHAN, 512); // 50% duty
        ledc_update_duty(LEDC_LOW_SPEED_MODE, BUZZER_LEDC_CHAN);
        vTaskDelay(pdMS_TO_TICKS(duration_ms));
        ledc_set_duty(LEDC_LOW_SPEED_MODE, BUZZER_LEDC_CHAN, 0);   // Stop Tone
        ledc_update_duty(LEDC_LOW_SPEED_MODE, BUZZER_LEDC_CHAN);
    } else {
        vTaskDelay(pdMS_TO_TICKS(duration_ms));
    }
}

// --- Servo PWM ---
void servo_init(void) {
    ledc_timer_config_t timer = {
        .speed_mode      = LEDC_LOW_SPEED_MODE,
        .timer_num       = SERVO_LEDC_TIMER,
        .duty_resolution = LEDC_TIMER_16_BIT,
        .freq_hz         = 50,
        .clk_cfg         = LEDC_AUTO_CLK,
    };
    ledc_timer_config(&timer);

    ledc_channel_config_t channel = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .channel    = SERVO_LEDC_CHAN,
        .timer_sel  = SERVO_LEDC_TIMER,
        .gpio_num   = SERVO_GPIO,
        .duty       = SERVO_MIN_TICKS,
        .hpoint     = 0,
    };
    ledc_channel_config(&channel);
}

void servo_set_angle(int angle) {
    if (angle < 0) angle = 0;
    if (angle > 180) angle = 180;
    uint32_t ticks = SERVO_MIN_TICKS + ((SERVO_MAX_TICKS - SERVO_MIN_TICKS) * angle / 180);
    ledc_set_duty(LEDC_LOW_SPEED_MODE, SERVO_LEDC_CHAN, ticks);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, SERVO_LEDC_CHAN);
}

// --- Digital Relay ---
void relay_init(void) {
    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << RELAY_OUT1_GPIO),
        .mode         = GPIO_MODE_OUTPUT,
        .pull_up_en   = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type    = GPIO_INTR_DISABLE
    };
    gpio_config(&io_conf);
    gpio_set_level(RELAY_OUT1_GPIO, 0);
}

// ==========================================
// 4. BUTTON ISR & TASKS
// ==========================================
static void IRAM_ATTR gpio_isr_handler(void *arg) {
    uint32_t gpio_num = (uint32_t)arg;
    xQueueSendFromISR(button_evt_queue, &gpio_num, NULL);
}

void button_init(void) {
    button_evt_queue = xQueueCreate(10, sizeof(uint32_t));

    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << BTN_SW1_GPIO) | (1ULL << BTN_SW2_GPIO),
        .mode         = GPIO_MODE_INPUT,
        .pull_up_en   = GPIO_PULLUP_ENABLE,
        .intr_type    = GPIO_INTR_NEGEDGE, // Trigger on button press
    };
    gpio_config(&io_conf);

    gpio_install_isr_service(ESP_INTR_FLAG_DEFAULT);
    gpio_isr_handler_add(BTN_SW1_GPIO, gpio_isr_handler, (void *)BTN_SW1_GPIO);
    gpio_isr_handler_add(BTN_SW2_GPIO, gpio_isr_handler, (void *)BTN_SW2_GPIO);
}

// ==========================================
// 5. FREERTOS SYSTEM TASKS
// ==========================================

/**
 * @brief Task: Handles button events gracefully.
 */
void button_task(void *pvParameters) {
    uint32_t io_num;
    int current_servo_angle = 0;

    while (1) {
        // Wait for a button event (blocking until ISR sends data)
        if (xQueueReceive(button_evt_queue, &io_num, portMAX_DELAY)) {
            if (io_num == BTN_SW1_GPIO) {
                ESP_LOGI(TAG, "SW1 Pressed! Moving Servo & Matrix Squint");
                
                matrix_draw(PATTERN_SQUINT);
                play_tone(2000, 100);
                
                current_servo_angle = (current_servo_angle == 0) ? 90 : 0;
                servo_set_angle(current_servo_angle);
                
                matrix_draw(PATTERN_SMILEY);
            } 
            else if (io_num == BTN_SW2_GPIO) {
                ESP_LOGI(TAG, "SW2 Pressed! Returning Servo");
                play_tone(1500, 100);
                
                current_servo_angle = 0;
                servo_set_angle(current_servo_angle);
            }
        }
    }
}

/**
 * @brief Task: Polls sensors and applies environmental logic.
 */
void sensor_logic_task(void *pvParameters) {
    while (1) {
        float temp_c = read_lm73();
        int ldr_val = read_ldr();

        ESP_LOGI(TAG, "Temp: %.2f C | LDR Raw: %d", temp_c, ldr_val);

        // Logic 1: LDR Dark Auto-Relay (e.g., Turn on outdoor light)
        if (ldr_val > 2500) { // Assuming > 2500 is dark
            gpio_set_level(RELAY_OUT1_GPIO, 1);
        } else {
            gpio_set_level(RELAY_OUT1_GPIO, 0);
        }

        // Logic 2: Over-Temp Alarm (e.g. > 35.0 C)
        if (temp_c > 35.0f) {
            ESP_LOGW(TAG, "OVER-TEMPERATURE ALERT!");
            play_tone(3000, 200); // Beep!
        }

        // Delay 1000 ms before next poll - Essential for Watchdog
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}

// ==========================================
// 6. MAIN APPLICATION ENTRY
// ==========================================
void app_main(void) {
    ESP_LOGI(TAG, "--- KidBright32iA Full System Demo Booting ---");

    // Initialize ALL Hardware using individual bus logic
    matrix_init();     // I2C_0
    temp_sensor_init();// I2C_1
    ldr_init();        // ADC1
    buzzer_init();     // PWM LEDC 0
    servo_init();      // PWM LEDC 1
    relay_init();      // Digital
    button_init();     // ISR Queue

    // Start FreeRTOS Tasks
    // Parameter Format: (Task function, Name, Stack Size (Words), Param, Priority, Handle)
    xTaskCreate(button_task, "button_evt_task", 4096, NULL, 10, NULL);
    xTaskCreate(sensor_logic_task, "sensor_logic", 4096, NULL, 5, NULL);

    ESP_LOGI(TAG, "--- System Initialization Complete ---");
}
