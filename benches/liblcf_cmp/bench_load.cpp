#include "lcf/ldb/reader.h"
#include "lcf/lmu/reader.h"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <numeric>
#include <sstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;
using clock_type = std::chrono::steady_clock;

namespace {

std::string read_file(const fs::path& path) {
    std::ifstream f(path, std::ios::binary);
    std::ostringstream ss;
    ss << f.rdbuf();
    return ss.str();
}

struct Stats {
    double min_ms, mean_ms, median_ms, max_ms, stddev_ms;
};

Stats summarize(std::vector<double>& samples_ms) {
    std::sort(samples_ms.begin(), samples_ms.end());
    double sum = std::accumulate(samples_ms.begin(), samples_ms.end(), 0.0);
    double mean = sum / static_cast<double>(samples_ms.size());
    double sq_sum = 0.0;
    for (double v : samples_ms)
        sq_sum += (v - mean) * (v - mean);
    double stddev = std::sqrt(sq_sum / static_cast<double>(samples_ms.size()));
    double median = samples_ms[samples_ms.size() / 2];
    return Stats{ samples_ms.front(), mean, median, samples_ms.back(), stddev };
}

void print_stats(const char* label, const Stats& s, size_t iters) {
    std::printf(
        "%-24s iters=%-6zu min=%.4fms mean=%.4fms median=%.4fms "
        "max=%.4fms stddev=%.4fms\n",
        label,
        iters,
        s.min_ms,
        s.mean_ms,
        s.median_ms,
        s.max_ms,
        s.stddev_ms
    );
}

constexpr int WARMUP = 20;
constexpr int ITERS = 300;

void bench_ldb(const fs::path& path) {
    std::string bytes = read_file(path);

    for (int i = 0; i < WARMUP; i++) {
        std::istringstream iss(bytes);
        auto db = lcf::LDB_Reader::Load(iss, "");
        if (!db) {
            std::cerr << "warmup load failed\n";
            std::exit(1);
        }
    }

    std::vector<double> samples;
    samples.reserve(ITERS);
    for (int i = 0; i < ITERS; i++) {
        auto t0 = clock_type::now();
        std::istringstream iss(bytes);
        auto db = lcf::LDB_Reader::Load(iss, "");
        auto t1 = clock_type::now();
        if (!db) {
            std::cerr << "load failed\n";
            std::exit(1);
        }
        samples.push_back(std::chrono::duration<double, std::milli>(t1 - t0).count());
    }

    print_stats("ldb (single database)", summarize(samples), ITERS);
}

void bench_maps(const fs::path& dir) {
    std::vector<std::string> map_bytes;
    for (const auto& entry : fs::directory_iterator(dir)) {
        if (entry.path().extension() == ".lmu") {
            map_bytes.push_back(read_file(entry.path()));
        }
    }
    if (map_bytes.empty()) {
        std::cerr << "no .lmu files found in " << dir << "\n";
        std::exit(1);
    }

    auto load_all = [&]() {
        for (auto& bytes : map_bytes) {
            std::istringstream iss(bytes);
            auto map = lcf::LMU_Reader::Load(iss, "");
            if (!map) {
                std::cerr << "map load failed\n";
                std::exit(1);
            }
        }
    };

    for (int i = 0; i < WARMUP / 4 + 1; i++)
        load_all();

    constexpr int MAP_PASSES = 30;
    std::vector<double> samples;
    samples.reserve(MAP_PASSES);
    for (int i = 0; i < MAP_PASSES; i++) {
        auto t0 = clock_type::now();
        load_all();
        auto t1 = clock_type::now();
        samples.push_back(std::chrono::duration<double, std::milli>(t1 - t0).count());
    }

    std::printf("(%zu map files per pass)\n", map_bytes.size());
    print_stats("maps (one full pass)", summarize(samples), MAP_PASSES);

    // Also report per-file average, for a number directly comparable to the ldb
    // line above.
    std::vector<double> per_file;
    per_file.reserve(samples.size());
    for (double v : samples)
        per_file.push_back(v / static_cast<double>(map_bytes.size()));
    print_stats("maps (per file)", summarize(per_file), MAP_PASSES);
}

}  // namespace

int main(int argc, char** argv) {
    if (argc < 3) {
        std::cerr << "usage: bench_load <ldb|maps> <path>\n";
        return 2;
    }
    std::string mode = argv[1];
    fs::path path = argv[2];

    if (mode == "ldb") {
        bench_ldb(path);
    } else if (mode == "maps") {
        bench_maps(path);
    } else {
        std::cerr << "unknown mode: " << mode << "\n";
        return 2;
    }
    return 0;
}
