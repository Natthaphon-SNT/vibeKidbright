#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

void app_main(void) {
    printf("Hello from vibeKidbright!\n");
    while (1) {
        printf("Heartbeat...\n");
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}
