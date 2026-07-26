{-# LANGUAGE DeriveFunctor #-}
{-# LANGUAGE LambdaCase #-}

module Pipeline where

import Control.Concurrent.Async (mapConcurrently)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

data Result a
  = Success a
  | Failure String
  deriving (Eq, Show, Functor)

data Job a = Job
  { jobName :: String
  , jobInput :: a
  } deriving (Eq, Show, Functor)

class Runnable task where
  run :: task a -> IO (Result a)

instance Runnable Job where
  run job
    | null (jobName job) =
        pure (Failure "missing name")
    | otherwise =
        pure (Success (jobInput job))

partitionResults
  :: [Result a]
  -> ([String], [a])
partitionResults = foldr step ([], [])
  where
    step result (errors, values) =
      case result of
        Failure message ->
          (message : errors, values)
        Success value ->
          (errors, value : values)

runAll
  :: Runnable task
  => [task a]
  -> IO (Map String a)
runAll jobs = do
  results <- mapConcurrently run jobs
  let (_, values) = partitionResults results
      names = ["job-" <> show n | n <- [1 :: Int ..]]
  pure (Map.fromList (zip names values))

describe :: Result a -> String
describe = \case
  Success _ -> "completed"
  Failure message -> "failed: " <> message
