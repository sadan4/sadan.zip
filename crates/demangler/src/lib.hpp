#pragma once
#include "rust/cxx.h"
#include <memory>
#include <string>

std::unique_ptr<std::string> demangle_all(rust::Str input);