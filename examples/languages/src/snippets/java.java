import java.time.Instant;
import java.util.List;
import java.util.concurrent.Executors;

sealed interface Event
    permits Started, Progress, Finished {}

record Started(String task, Instant at)
    implements Event {}

record Progress(String task, int percent)
    implements Event {
    Progress {
        if (percent < 0 || percent > 100) {
            throw new IllegalArgumentException("percent");
        }
    }
}

record Finished(String task, boolean ok)
    implements Event {}

final class EventLog<T extends Event> {
    private final List<T> events;

    EventLog(List<T> events) {
        this.events = List.copyOf(events);
    }

    List<String> messages() {
        return events.stream()
            .map(EventLog::describe)
            .toList();
    }

    private static String describe(Event event) {
        return switch (event) {
            case Started(var task, var at) ->
                "%s started at %s".formatted(task, at);
            case Progress(var task, var percent)
                when percent == 100 ->
                task + " is ready";
            case Progress(var task, var percent) ->
                "%s: %d%%".formatted(task, percent);
            case Finished(var task, var ok) when ok ->
                task + " finished";
            case Finished(var task, var ok) ->
                task + " failed";
        };
    }
}

class Main {
    public static void main(String[] args)
        throws Exception {
        try (var tasks =
                 Executors.newVirtualThreadPerTaskExecutor()) {
            var future = tasks.submit(() ->
                new EventLog<>(List.of(
                    new Started("index", Instant.now()),
                    new Progress("index", 100),
                    new Finished("index", true)
                )).messages()
            );
            future.get().forEach(System.out::println);
        }
    }
}
