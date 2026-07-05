cmake_minimum_required(VERSION 3.21)

project(AresQtWorkstation VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Quick)

qt_standard_project_setup(REQUIRES 6.5)

qt_add_executable(ares_qt_workstation
    src/main.cpp
)

set_source_files_properties(qml/Main.qml
    PROPERTIES
        QT_RESOURCE_ALIAS Main.qml
)

qt_add_qml_module(ares_qt_workstation
    URI AresNetGuard.Workstation
    VERSION 1.0
    QML_FILES
        qml/Main.qml
)

target_link_libraries(ares_qt_workstation
    PRIVATE
        Qt6::Quick
)
