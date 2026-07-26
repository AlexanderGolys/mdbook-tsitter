#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

typedef enum {
    EVENT_DATA,
    EVENT_ERROR,
    EVENT_CLOSED,
} EventKind;

typedef struct {
    const char *data;
    size_t length;
} Slice;

typedef struct {
    EventKind kind;
    union {
        Slice data;
        int error_code;
    };
} Event;

typedef bool (*EventHandler)(
    const Event *event,
    void *context
);

typedef struct {
    Event *items;
    size_t length;
    size_t capacity;
} EventQueue;

static bool queue_push(
    EventQueue *queue,
    Event event
) {
    if (queue->length == queue->capacity) {
        size_t capacity =
            queue->capacity ? queue->capacity * 2 : 8;
        Event *items = realloc(
            queue->items,
            capacity * sizeof(*items)
        );
        if (items == NULL) {
            return false;
        }
        queue->items = items;
        queue->capacity = capacity;
    }

    queue->items[queue->length++] = event;
    return true;
}

static void queue_drain(
    EventQueue *queue,
    EventHandler handle,
    void *context
) {
    for (size_t i = 0; i < queue->length; ++i) {
        if (!handle(&queue->items[i], context)) {
            break;
        }
    }
    queue->length = 0;
}

static bool print_event(
    const Event *event,
    void *context
) {
    FILE *output = context;
    switch (event->kind) {
    case EVENT_DATA:
        fprintf(
            output,
            "%.*s\n",
            (int)event->data.length,
            event->data.data
        );
        return true;
    case EVENT_ERROR:
        fprintf(output, "error %d\n", event->error_code);
        return false;
    case EVENT_CLOSED:
        return false;
    }
    return false;
}

int main(void) {
    EventQueue queue = {0};
    queue_push(&queue, (Event){
        .kind = EVENT_DATA,
        .data = {"ready", 5},
    });
    queue_drain(&queue, print_event, stdout);
    free(queue.items);
}
