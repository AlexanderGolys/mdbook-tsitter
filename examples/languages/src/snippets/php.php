<?php

declare(strict_types=1);

namespace App\Jobs;

use Attribute;
use Generator;

#[Attribute(Attribute::TARGET_CLASS)]
final readonly class Queue
{
    public function __construct(
        public string $name,
        public int $retries = 3,
    ) {}
}

enum State: string
{
    case Queued = 'queued';
    case Running = 'running';
    case Done = 'done';
    case Failed = 'failed';

    public function terminal(): bool
    {
        return match ($this) {
            self::Done, self::Failed => true,
            default => false,
        };
    }
}

interface Job
{
    public function id(): string;
    public function run(Context $ctx): State;
}

#[Queue('reports', retries: 5)]
final readonly class BuildReport implements Job
{
    public function __construct(
        private string $reportId,
        private array $records,
    ) {}

    public function id(): string
    {
        return $this->reportId;
    }

    public function rows(): Generator
    {
        foreach ($this->records as $record) {
            yield [
                'name' => $record['name'],
                'score' => (float) $record['score'],
            ];
        }
    }

    public function run(Context $ctx): State
    {
        $rows = [...$this->rows()];
        usort(
            $rows,
            fn(array $a, array $b): int =>
                $b['score'] <=> $a['score'],
        );

        return $ctx->write($this->id(), $rows)
            ? State::Done
            : State::Failed;
    }
}
