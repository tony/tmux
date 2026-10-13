#include "window/layout.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <gtest/gtest.h>
#include <utility>

namespace tmux_cxx {
namespace {
/** @brief Build existing panes by splitting the widest leaf without requiring programs. */
WindowLayout split_layout(std::size_t count, terminal::CellSize size = {24, 80}) {
    WindowLayout layout(PaneId{1}, size);
    for (std::size_t identity = 2; identity <= count; ++identity) {
        const auto panes = layout.panes();
        const auto widest = std::ranges::max_element(
            panes, {}, /** @brief Compare available horizontal splitting space. */
            [](const PaneGeometry &pane) { return pane.size.columns; });
        EXPECT_TRUE(layout.split(widest->pane, PaneId{identity}, PaneSplitOrientation::left_right));
    }
    return layout;
}

/** @brief Compare identity-correlated leaf geometry without requiring traversal order. */
void expect_geometry(const WindowLayout &layout, PaneId pane, std::uint32_t top, std::uint32_t left,
                     terminal::CellSize size) {
    const auto geometry = layout.pane(pane);
    ASSERT_TRUE(geometry);
    EXPECT_EQ(geometry->top, top);
    EXPECT_EQ(geometry->left, left);
    EXPECT_EQ(geometry->size.rows, size.rows);
    EXPECT_EQ(geometry->size.columns, size.columns);
}
} // namespace

/** @brief Match seven stock preset geometries while retaining the logical first main pane. */
TEST(WindowLayoutPresets, ThreePaneStockGeometry) {
    /** @brief Retain independently calculated stock geometry for one named arrangement. */
    struct Expected {
        WindowLayoutPreset preset;         ///< Named arrangement being verified.
        std::array<PaneGeometry, 3> panes; ///< Identity-correlated expected geometry.
    };
    const std::array expected{
        Expected{WindowLayoutPreset::even_horizontal,
                 {{{PaneId{1}, 0, 0, {24, 26}},
                   {PaneId{2}, 0, 27, {24, 26}},
                   {PaneId{3}, 0, 54, {24, 26}}}}},
        Expected{WindowLayoutPreset::even_vertical,
                 {{{PaneId{1}, 0, 0, {8, 80}},
                   {PaneId{2}, 9, 0, {7, 80}},
                   {PaneId{3}, 17, 0, {7, 80}}}}},
        Expected{WindowLayoutPreset::main_horizontal,
                 {{{PaneId{1}, 0, 0, {22, 80}},
                   {PaneId{2}, 23, 0, {1, 40}},
                   {PaneId{3}, 23, 41, {1, 39}}}}},
        Expected{WindowLayoutPreset::main_horizontal_mirrored,
                 {{{PaneId{1}, 2, 0, {22, 80}},
                   {PaneId{2}, 0, 0, {1, 40}},
                   {PaneId{3}, 0, 41, {1, 39}}}}},
        Expected{WindowLayoutPreset::main_vertical,
                 {{{PaneId{1}, 0, 0, {24, 78}},
                   {PaneId{2}, 0, 79, {12, 1}},
                   {PaneId{3}, 13, 79, {11, 1}}}}},
        Expected{WindowLayoutPreset::main_vertical_mirrored,
                 {{{PaneId{1}, 0, 2, {24, 78}},
                   {PaneId{2}, 0, 0, {12, 1}},
                   {PaneId{3}, 13, 0, {11, 1}}}}},
        Expected{WindowLayoutPreset::tiled,
                 {{{PaneId{1}, 0, 0, {11, 39}},
                   {PaneId{2}, 0, 40, {11, 40}},
                   {PaneId{3}, 12, 0, {12, 80}}}}},
    };
    const std::array order{PaneId{1}, PaneId{2}, PaneId{3}};
    for (const auto &arrangement : expected) {
        auto layout = split_layout(3);
        const auto changed = layout.arrange(arrangement.preset, order);
        ASSERT_TRUE(changed);
        EXPECT_TRUE(*changed);
        for (const auto &pane : arrangement.panes) {
            expect_geometry(layout, pane.pane, pane.top, pane.left, pane.size);
        }
        EXPECT_EQ(layout.size().rows, 24);
        EXPECT_EQ(layout.size().columns, 80);
        EXPECT_EQ(layout.arrange(arrangement.preset, order), false);
    }
}

/** @brief Preserve literal main-pane defaults when the window has room beyond their requested size.
 */
TEST(WindowLayoutPresets, MainSizesUseStockDefaults) {
    auto layout = split_layout(3, {50, 120});
    const std::array order{PaneId{1}, PaneId{2}, PaneId{3}};
    ASSERT_TRUE(layout.arrange(WindowLayoutPreset::main_horizontal, order));
    expect_geometry(layout, PaneId{1}, 0, 0, {24, 120});
    expect_geometry(layout, PaneId{2}, 25, 0, {25, 60});
    expect_geometry(layout, PaneId{3}, 25, 61, {25, 59});
    ASSERT_TRUE(layout.arrange(WindowLayoutPreset::main_vertical, order));
    expect_geometry(layout, PaneId{1}, 0, 0, {50, 80});
    expect_geometry(layout, PaneId{2}, 0, 81, {25, 39});
    expect_geometry(layout, PaneId{3}, 26, 81, {24, 39});
}

/** @brief Put tiled residuals in final panes and rows, including a partial final row. */
TEST(WindowLayoutPresets, TiledResidualsAndTwoPaneRows) {
    auto two = split_layout(2);
    const std::array two_order{PaneId{1}, PaneId{2}};
    ASSERT_TRUE(two.arrange(WindowLayoutPreset::tiled, two_order));
    expect_geometry(two, PaneId{1}, 0, 0, {11, 80});
    expect_geometry(two, PaneId{2}, 12, 0, {12, 80});
    EXPECT_EQ(two.arrange(WindowLayoutPreset::even_vertical, two_order), true);
    expect_geometry(two, PaneId{1}, 0, 0, {12, 80});
    expect_geometry(two, PaneId{2}, 13, 0, {11, 80});
    auto eight = split_layout(8);
    const std::array order{PaneId{1}, PaneId{2}, PaneId{3}, PaneId{4},
                           PaneId{5}, PaneId{6}, PaneId{7}, PaneId{8}};
    ASSERT_TRUE(eight.arrange(WindowLayoutPreset::tiled, order));
    expect_geometry(eight, PaneId{1}, 0, 0, {7, 26});
    expect_geometry(eight, PaneId{3}, 0, 54, {7, 26});
    expect_geometry(eight, PaneId{7}, 16, 0, {8, 26});
    expect_geometry(eight, PaneId{8}, 16, 27, {8, 53});
}

/** @brief Reapply a preset after manual resizing while rejecting invalid membership before changes.
 */
TEST(WindowLayoutPresets, RestoreAfterResizeAndRejectInvalidOrders) {
    auto layout = split_layout(3);
    const std::array order{PaneId{1}, PaneId{2}, PaneId{3}};
    ASSERT_TRUE(layout.arrange(WindowLayoutPreset::main_horizontal_mirrored, order));
    ASSERT_TRUE(layout.resize_pane(PaneId{2}, PaneResizeDirection::right, 5));
    EXPECT_EQ(layout.arrange(WindowLayoutPreset::main_horizontal_mirrored, order), true);
    EXPECT_EQ(layout.arrange(WindowLayoutPreset::main_horizontal_mirrored, order), false);
    for (const auto invalid : {std::uint32_t{7}, std::uint32_t{65536}}) {
        const auto rejected = layout.arrange(std::bit_cast<WindowLayoutPreset>(invalid), order);
        ASSERT_FALSE(rejected);
        EXPECT_EQ(rejected.error().code, TmuxErrorCode::invalid_argument);
    }
    const std::array duplicate{PaneId{1}, PaneId{1}, PaneId{3}};
    const std::array foreign{PaneId{1}, PaneId{2}, PaneId{99}};
    const std::array zero{PaneId{1}, PaneId{2}, PaneId{0}};
    const std::array missing{PaneId{1}, PaneId{2}};
    EXPECT_FALSE(layout.arrange(WindowLayoutPreset::even_vertical, duplicate));
    EXPECT_FALSE(layout.arrange(WindowLayoutPreset::even_vertical, foreign));
    EXPECT_FALSE(layout.arrange(WindowLayoutPreset::even_vertical, zero));
    EXPECT_FALSE(layout.arrange(WindowLayoutPreset::even_vertical, missing));
    EXPECT_FALSE(layout.arrange(WindowLayoutPreset::even_vertical, {}));
    EXPECT_EQ(layout.arrange(WindowLayoutPreset::main_horizontal_mirrored, order), false);
}

/** @brief Expand to leaf minima but reject layouts exceeding bounded terminal capacity. */
TEST(WindowLayoutPresets, MinimumExpansionAndCapacityRejection) {
    auto layout = split_layout(3);
    ASSERT_TRUE(layout.resize({1, 5}));
    const std::array order{PaneId{1}, PaneId{2}, PaneId{3}};
    ASSERT_TRUE(layout.arrange(WindowLayoutPreset::even_vertical, order));
    EXPECT_EQ(layout.size().rows, 5);
    EXPECT_EQ(layout.size().columns, 5);
    expect_geometry(layout, PaneId{3}, 4, 0, {1, 5});
    auto full = split_layout(256, {1, 511});
    std::array<PaneId, 256> all{};
    for (std::size_t index = 0; index < all.size(); ++index) {
        all[index] = PaneId{index + 1};
    }
    const auto rejected = full.arrange(WindowLayoutPreset::even_vertical, all);
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_EQ(full.size().rows, 1);
    EXPECT_EQ(full.size().columns, 511);
}

/** @brief Preserve singleton geometry for every named layout. */
TEST(WindowLayoutPresets, SingletonPreservesGeometry) {
    WindowLayout layout(PaneId{1}, {24, 80});
    const std::array order{PaneId{1}};
    for (const auto &entry : window_layout_presets) {
        EXPECT_EQ(layout.arrange(entry.preset, order), false);
    }
    expect_geometry(layout, PaneId{1}, 0, 0, {24, 80});
}
} // namespace tmux_cxx
