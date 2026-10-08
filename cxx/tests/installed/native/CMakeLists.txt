cmake_minimum_required(VERSION 4.4)
project(tmux_cxx_installed_consumer LANGUAGES CXX)

find_package(tmux_cxx 0.1 REQUIRED CONFIG)

add_executable(tmux_cxx_installed_consumer main.cpp)
target_link_libraries(tmux_cxx_installed_consumer PRIVATE tmux_cxx::server)
set_target_properties(tmux_cxx_installed_consumer PROPERTIES
  CXX_STANDARD 23
  CXX_STANDARD_REQUIRED YES
  CXX_EXTENSIONS NO)
