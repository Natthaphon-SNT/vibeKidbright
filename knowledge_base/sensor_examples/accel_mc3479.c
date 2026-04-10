/* [FILE: knowledge_base/sensor_examples/accel_mc3479.c]
 * 
 * ESP-IDF I2C Example for MC3479 3-Axis Accelerometer on KidBright32
 * 
 * Hardware Notes:
 * - Some revisions of KidBright32 use the mCube MC3479 Accelerometer.
 * - I2C Address: 0x6C
 * - Internal I2C Bus Pins: SDA = GPIO 4, SCL = GPIO 5
 * - I2C Speed: 100 kHz
 * 
 * AI Instruction Rules:
 * 1. ALWAYS use `i2c_master_write_read_device` for reading sensor registers. DO NOT use fragmented start/write/stop links unless completely unavoidable.
 * 2. MC3479 starts in standby! You MUST write 0x01 to register 0x07 (Mode) to wake it up before reading.
 * 3. The X and Y axes are swapped/inverted mechanically depending on exactly which KidBright board revision is used. The logic below adjusts it for a natural UP/DOWN/LEFT/RIGHT layout.
 */

#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"

#define I2C_MASTER_SCL_IO           5
#define I2C_MASTER_SDA_IO           4
#define I2C_MASTER_NUM              0
#define I2C_MASTER_FREQ_HZ          100000
#define I2C_TIMEOUT_MS              1000

#define MC3479_ADDR                 0x6C    

#define TILT_THRESHOLD              5000 

static const char *TAG = "MC3479_ACCEL";

static void i2c_master_init(void) {
    i2c_config_t conf = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C_MASTER_SDA_IO,
        .scl_io_num = I2C_MASTER_SCL_IO,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C_MASTER_FREQ_HZ,
    };
    i2c_param_config(I2C_MASTER_NUM, &conf);
    i2c_driver_install(I2C_MASTER_NUM, conf.mode, 0, 0, 0);
}

static esp_err_t mc3479_init(void) {
    uint8_t write_buf[2] = {0x07, 0x01}; // Register 0x07 (Mode), 0x01 (Wake)
    esp_err_t err = i2c_master_write_to_device(I2C_MASTER_NUM, MC3479_ADDR, write_buf, 2, I2C_TIMEOUT_MS / portTICK_PERIOD_MS);
    if (err == ESP_OK) {
        ESP_LOGI(TAG, "MC3479 Wake-up OK!");
    } else {
        ESP_LOGE(TAG, "MC3479 Wake-up Failed!");
    }
    return err;
}

void app_main(void) {
    ESP_LOGI(TAG, "Initializing I2C...");
    i2c_master_init();
    vTaskDelay(pdMS_TO_TICKS(100));

    if (mc3479_init() != ESP_OK) {
        ESP_LOGE(TAG, "Halt. Sensor not found.");
        return;
    }

    uint8_t reg_addr = 0x0D; // X_LSB Register
    uint8_t data[6];

    while (1) {
        esp_err_t ret = i2c_master_write_read_device(I2C_MASTER_NUM, MC3479_ADDR, &reg_addr, 1, data, 6, I2C_TIMEOUT_MS / portTICK_PERIOD_MS);

        if (ret == ESP_OK) {
            int16_t x = (int16_t)((data[1] << 8) | data[0]);
            int16_t y = (int16_t)((data[3] << 8) | data[2]);

            // สลับทิศทางให้ตรงกับการวางชิปบนบอร์ด KidBright
            if (y > TILT_THRESHOLD) {
                ESP_LOGI(TAG, "Direction: DOWN  (Y: %d)", y);
            } 
            else if (y < -TILT_THRESHOLD) {
                ESP_LOGI(TAG, "Direction: UP    (Y: %d)", y);
            } 
            else if (x > TILT_THRESHOLD) {
                ESP_LOGI(TAG, "Direction: LEFT  (X: %d)", x);
            } 
            else if (x < -TILT_THRESHOLD) {
                ESP_LOGI(TAG, "Direction: RIGHT (X: %d)", x);
            } 
            else {
                // ระนาบปกติ
            }
        } else {
            ESP_LOGW(TAG, "Read error");
        }
        
        vTaskDelay(pdMS_TO_TICKS(200));
    }
}
