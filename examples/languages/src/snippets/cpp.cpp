#include <algorithm>
#include <concepts>
#include <iostream>
#include <memory>
#include <ranges>
#include <string>
#include <variant>
#include <vector>

struct Text {
    std::string value;
};

struct Number {
    double value;
};

using Cell = std::variant<Text, Number>;

template<typename T>
concept Printable = requires(
    std::ostream& out,
    const T& value
) {
    { out << value } -> std::same_as<std::ostream&>;
};

class Table {
public:
    void add(Cell cell) {
        cells_.push_back(std::move(cell));
    }

    auto numbers() const {
        return cells_
            | std::views::filter([](const Cell& cell) {
                return std::holds_alternative<Number>(
                    cell
                );
            })
            | std::views::transform([](const Cell& cell) {
                return std::get<Number>(cell).value;
            });
    }

private:
    std::vector<Cell> cells_;
};

template<std::ranges::input_range R>
requires std::convertible_to<
    std::ranges::range_value_t<R>,
    double
>
double average(R&& values) {
    double sum = 0.0;
    std::size_t count = 0;
    for (double value : values) {
        sum += value;
        ++count;
    }
    return count ? sum / count : 0.0;
}

int main() {
    auto table = std::make_unique<Table>();
    table->add(Text{"latency"});
    table->add(Number{18.4});
    table->add(Number{21.2});

    std::cout << average(table->numbers()) << '\n';
}
