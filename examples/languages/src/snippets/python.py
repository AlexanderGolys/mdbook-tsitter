from collections.abc import AsyncIterator
from dataclasses import dataclass, field
from enum import StrEnum, auto
from typing import Protocol, TypeVar
import asyncio

T = TypeVar("T")


class State(StrEnum):
    QUEUED = auto()
    RUNNING = auto()
    DONE = auto()
    FAILED = auto()


@dataclass(slots=True)
class Job:
    name: str
    command: tuple[str, ...]
    state: State = State.QUEUED
    output: list[str] = field(default_factory=list)


class Store(Protocol[T]):
    async def save(self, key: str, value: T) -> None: ...
    async def load(self, key: str) -> T | None: ...


async def run(job: Job) -> AsyncIterator[str]:
    job.state = State.RUNNING
    proc = await asyncio.create_subprocess_exec(
        *job.command,
        stdout=asyncio.subprocess.PIPE,
    )
    assert proc.stdout is not None

    async for raw in proc.stdout:
        line = raw.decode().rstrip()
        job.output.append(line)
        yield line

    code = await proc.wait()
    job.state = State.DONE if code == 0 else State.FAILED


async def supervise(
    jobs: list[Job],
    store: Store[Job],
) -> dict[State, int]:
    async with asyncio.TaskGroup() as group:
        for job in jobs:
            group.create_task(store.save(job.name, job))

    counts = {state: 0 for state in State}
    for job in jobs:
        match job.state:
            case State.FAILED if job.output:
                print(f"{job.name}: {job.output[-1]}")
            case State.DONE:
                print(f"{job.name}: complete")
            case _:
                pass
        counts[job.state] += 1
    return counts
