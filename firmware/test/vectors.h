#pragma once
// Loads protocol golden vectors (protocol/vectors/*.hex) for native tests.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

inline std::vector<uint8_t> loadVector(const char* name) {
  const std::string path = std::string(EAD_VECTORS_DIR) + "/" + name;
  FILE* f = std::fopen(path.c_str(), "r");
  if (f == nullptr) {
    std::fprintf(stderr, "cannot open vector %s\n", path.c_str());
    std::exit(2);
  }
  std::vector<uint8_t> bytes;
  char line[512];
  while (std::fgets(line, sizeof line, f) != nullptr) {
    if (line[0] == '#') continue;
    const char* p = line;
    while (*p != '\0') {
      while (*p == ' ' || *p == '\n' || *p == '\r') p++;
      if (*p == '\0') break;
      char hex[3] = {p[0], p[1], '\0'};
      bytes.push_back(uint8_t(std::strtoul(hex, nullptr, 16)));
      p += 2;
    }
  }
  std::fclose(f);
  return bytes;
}
