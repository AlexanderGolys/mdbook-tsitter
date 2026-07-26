local Stream = {}
Stream.__index = Stream

function Stream.new(source)
  return setmetatable({
    source = source,
    steps = {},
  }, Stream)
end

function Stream:map(fn)
  self.steps[#self.steps + 1] = function(value)
    return true, fn(value)
  end
  return self
end

function Stream:filter(predicate)
  self.steps[#self.steps + 1] = function(value)
    return predicate(value), value
  end
  return self
end

function Stream:iter()
  return coroutine.wrap(function()
    for value in self.source do
      local keep = true
      local current = value
      for _, step in ipairs(self.steps) do
        keep, current = step(current)
        if not keep then break end
      end
      if keep then coroutine.yield(current) end
    end
  end)
end

local function values(items)
  local index = 0
  return function()
    index = index + 1
    return items[index]
  end
end

local stream = Stream.new(values({ 1, 2, 3, 4, 5 }))
  :filter(function(n) return n % 2 == 1 end)
  :map(function(n) return n * n end)

local ok, err = pcall(function()
  for value in stream:iter() do
    print(("square: %d"):format(value))
  end
end)

if not ok then
  io.stderr:write(err, "\n")
end
