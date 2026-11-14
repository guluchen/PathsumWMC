// Browser entry point for the pinned GPMC search engine. MIT licensed.
#include <fstream>
#include <iostream>
#include "utils/Options.h"
#include "core/Config.h"
#include "core/Counter.h"
#include "c-mpfr/ComplexMPFR.h"
using namespace GPMC;
using namespace Glucose;
int main(int argc, char** argv) {
    try {
        parseOptions(argc, argv, true);
        if (argc != 2) { std::cerr << "Expected one CNF file\n"; return 2; }
        Configuration config;
        if (config.cntr.mode != WMC) return 2;
        Counter<Complex> solver(config);
        std::ifstream input(argv[1]);
        if (!input) return 2;
        solver.load(input);
        if (!solver.preprocess()) {
            solver.setExtraVarScore();
            if (!solver.countModels()) return 3;
        }
        std::cout << "c s exact arb frac " << solver.getMC() << std::endl;
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << std::endl; return 1;
    }
}
