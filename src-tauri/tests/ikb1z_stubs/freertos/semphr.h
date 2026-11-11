#ifndef IKB_TEST_SEMPHR_H
#define IKB_TEST_SEMPHR_H
#include "FreeRTOS.h"
typedef void *SemaphoreHandle_t;
BaseType_t xSemaphoreTake(SemaphoreHandle_t lock, TickType_t ticks);
BaseType_t xSemaphoreGive(SemaphoreHandle_t lock);
#endif
