#ifndef IKB_TEST_FREERTOS_H
#define IKB_TEST_FREERTOS_H
#include <stdint.h>
typedef uint32_t TickType_t;
typedef int BaseType_t;
#define pdTRUE 1
#define pdFALSE 0
#define configTICK_RATE_HZ 1000u
#define pdMS_TO_TICKS(ms) ((TickType_t)(ms))
#endif
