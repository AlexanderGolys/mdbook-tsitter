package pipeline

import (
	"context"
	"errors"
	"sync"
)

type Item interface {
	Key() string
}

type Store[T Item] interface {
	Load(context.Context, string) (T, error)
	Save(context.Context, T) error
}

type Result[T any] struct {
	Value T
	Err   error
}

func Map[A, B any](
	ctx context.Context,
	input <-chan A,
	workers int,
	fn func(context.Context, A) (B, error),
) <-chan Result[B] {
	output := make(chan Result[B])
	var group sync.WaitGroup

	group.Add(workers)
	for range workers {
		go func() {
			defer group.Done()
			for value := range input {
				item, err := fn(ctx, value)
				select {
				case output <- Result[B]{
					Value: item,
					Err:   err,
				}:
				case <-ctx.Done():
					return
				}
			}
		}()
	}

	go func() {
		group.Wait()
		close(output)
	}()
	return output
}

func Collect[T any](
	ctx context.Context,
	input <-chan Result[T],
) ([]T, error) {
	var values []T
	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case result, ok := <-input:
			if !ok {
				return values, nil
			}
			if result.Err != nil {
				return nil, errors.Join(
					result.Err,
					ctx.Err(),
				)
			}
			values = append(values, result.Value)
		}
	}
}
