import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    objectName: "workstationShellRoot"
    width: 1440
    height: 900
    minimumWidth: 1180
    minimumHeight: 720
    visible: true
    title: "ARES NetGuard-ML Workstation"

    readonly property color backgroundColor: "#101312"
    readonly property color panelColor: "#1b211f"
    readonly property color panelDeepColor: "#151918"
    readonly property color borderColor: "#31413d"
    readonly property color mutedTextColor: "#aab7b2"
    readonly property color primaryTextColor: "#edf4f1"
    readonly property color accentColor: "#22a69a"
    readonly property color amberColor: "#d49a34"
    readonly property color redColor: "#df5b4f"
    readonly property color greenColor: "#45b879"

    readonly property var navigationItems: [
        "Workspace",
        "Model Disagreement",
        "Evidence Graph",
        "Investigation",
        "Detection Candidates",
        "Model Registry"
    ]

    readonly property var evidenceRows: [
        {
            "entity": "asset-alpha",
            "window": "2026-01-01T00:05Z",
            "consensus": 0.91,
            "disagreement": 0.08,
            "models": "IForest, ECOD, River HST",
            "evidence": "Destination diversity and DNS failure ratio elevated",
            "state": "Triage"
        },
        {
            "entity": "asset-beta",
            "window": "2026-01-01T00:10Z",
            "consensus": 0.58,
            "disagreement": 0.71,
            "models": "Graph novelty dissenting",
            "evidence": "Known service relationship refutes bytes-outlier signal",
            "state": "Compare"
        },
        {
            "entity": "asset-gamma",
            "window": "2026-01-01T00:15Z",
            "consensus": 0.73,
            "disagreement": 0.22,
            "models": "Suricata fixture, River HST",
            "evidence": "Synthetic alert severity aligns with online baseline",
            "state": "Monitor"
        },
        {
            "entity": "asset-delta",
            "window": "2026-01-01T00:20Z",
            "consensus": 0.34,
            "disagreement": 0.64,
            "models": "Representation risk only",
            "evidence": "Rare token cluster lacks corroborating graph evidence",
            "state": "Review"
        }
    ]

    function riskColor(value) {
        if (value >= 0.8) {
            return redColor;
        }
        if (value >= 0.6) {
            return amberColor;
        }
        return greenColor;
    }

    Rectangle {
        anchors.fill: parent
        color: root.backgroundColor

        RowLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                objectName: "leftNavigation"
                Layout.preferredWidth: 248
                Layout.fillHeight: true
                color: root.panelDeepColor
                border.color: root.borderColor
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 18

                    ColumnLayout {
                        spacing: 4

                        Label {
                            text: "ARES NetGuard-ML"
                            color: root.primaryTextColor
                            font.pixelSize: 20
                            font.bold: true
                        }

                        Label {
                            text: "AI-NDR Workstation"
                            color: root.mutedTextColor
                            font.pixelSize: 12
                        }
                    }

                    ColumnLayout {
                        objectName: "navigationList"
                        Layout.fillWidth: true
                        spacing: 6

                        Repeater {
                            model: root.navigationItems

                            delegate: Button {
                                id: navButton
                                objectName: modelData.replace(/ /g, "") + "Navigation"
                                Layout.fillWidth: true
                                implicitHeight: 38
                                text: modelData
                                hoverEnabled: true
                                highlighted: index === 1

                                contentItem: Label {
                                    text: navButton.text
                                    color: navButton.highlighted
                                        ? root.primaryTextColor
                                        : root.mutedTextColor
                                    font.pixelSize: 13
                                    font.bold: navButton.highlighted
                                    verticalAlignment: Text.AlignVCenter
                                    elide: Text.ElideRight
                                }

                                background: Rectangle {
                                    radius: 4
                                    color: navButton.highlighted
                                        ? Qt.rgba(0.13, 0.65, 0.60, 0.22)
                                        : navButton.hovered
                                            ? "#202927"
                                            : "transparent"
                                    border.color: navButton.highlighted
                                        ? root.accentColor
                                        : "transparent"
                                    border.width: 1
                                }
                            }
                        }
                    }

                    Item {
                        Layout.fillHeight: true
                    }

                    Rectangle {
                        objectName: "localFixtureStatus"
                        Layout.fillWidth: true
                        implicitHeight: 84
                        radius: 4
                        color: "#202724"
                        border.color: root.borderColor

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 4

                            Label {
                                text: "Source"
                                color: root.mutedTextColor
                                font.pixelSize: 11
                            }

                            Label {
                                text: "Static synthetic fixture"
                                color: root.primaryTextColor
                                font.pixelSize: 13
                                font.bold: true
                            }

                            Label {
                                text: "Local scaffold"
                                color: root.accentColor
                                font.pixelSize: 12
                            }
                        }
                    }
                }
            }

            Rectangle {
                objectName: "modelEvidenceWorkspace"
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: root.backgroundColor

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 22
                    spacing: 16

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 16

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Label {
                                text: "Model Disagreement"
                                color: root.primaryTextColor
                                font.pixelSize: 26
                                font.bold: true
                            }

                            Label {
                                text: "Synthetic review window / 5m entity features"
                                color: root.mutedTextColor
                                font.pixelSize: 13
                            }
                        }

                        Rectangle {
                            implicitWidth: 142
                            implicitHeight: 36
                            radius: 4
                            color: "#1f2a27"
                            border.color: root.borderColor

                            Label {
                                anchors.centerIn: parent
                                text: "Evaluation v0"
                                color: root.primaryTextColor
                                font.pixelSize: 13
                                font.bold: true
                            }
                        }
                    }

                    RowLayout {
                        objectName: "modelDisagreementSummary"
                        Layout.fillWidth: true
                        spacing: 10

                        Repeater {
                            model: [
                                { "label": "Consensus risk", "value": "0.91", "color": root.redColor },
                                { "label": "Largest dissent", "value": "0.71", "color": root.amberColor },
                                { "label": "Rows in window", "value": "4", "color": root.accentColor }
                            ]

                            delegate: Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: 84
                                radius: 4
                                color: root.panelColor
                                border.color: root.borderColor

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 14
                                    spacing: 6

                                    Label {
                                        text: modelData.label
                                        color: root.mutedTextColor
                                        font.pixelSize: 12
                                    }

                                    Label {
                                        text: modelData.value
                                        color: modelData.color
                                        font.pixelSize: 24
                                        font.bold: true
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        objectName: "modelEvidenceMatrix"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        radius: 4
                        color: root.panelColor
                        border.color: root.borderColor

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 10

                            RowLayout {
                                Layout.fillWidth: true

                                Label {
                                    Layout.fillWidth: true
                                    text: "Entity Evidence Matrix"
                                    color: root.primaryTextColor
                                    font.pixelSize: 17
                                    font.bold: true
                                }

                                Label {
                                    text: "model_score_row.v0"
                                    color: root.mutedTextColor
                                    font.pixelSize: 12
                                }
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: 32
                                color: "#111715"
                                radius: 3

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 12
                                    anchors.rightMargin: 12
                                    spacing: 12

                                    Repeater {
                                        model: [
                                            { "label": "Entity", "width": 116 },
                                            { "label": "Consensus", "width": 96 },
                                            { "label": "Dissent", "width": 82 },
                                            { "label": "Model signal", "width": 178 },
                                            { "label": "Evidence", "width": 1 },
                                            { "label": "State", "width": 88 }
                                        ]

                                        delegate: Label {
                                            Layout.preferredWidth: modelData.width > 1
                                                ? modelData.width
                                                : 160
                                            Layout.fillWidth: modelData.width === 1
                                            text: modelData.label
                                            color: root.mutedTextColor
                                            font.pixelSize: 11
                                            font.bold: true
                                            elide: Text.ElideRight
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }
                                }
                            }

                            Repeater {
                                model: root.evidenceRows

                                delegate: Rectangle {
                                    required property var modelData

                                    Layout.fillWidth: true
                                    implicitHeight: 64
                                    radius: 4
                                    color: index === 0 ? "#222b28" : "#181f1d"
                                    border.color: index === 0 ? root.accentColor : root.borderColor

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 12
                                        anchors.rightMargin: 12
                                        spacing: 12

                                        Label {
                                            Layout.preferredWidth: 116
                                            text: modelData.entity
                                            color: root.primaryTextColor
                                            font.pixelSize: 13
                                            font.bold: true
                                            elide: Text.ElideRight
                                        }

                                        Label {
                                            Layout.preferredWidth: 96
                                            text: modelData.consensus.toFixed(2)
                                            color: root.riskColor(modelData.consensus)
                                            font.pixelSize: 13
                                            font.bold: true
                                        }

                                        Label {
                                            Layout.preferredWidth: 82
                                            text: modelData.disagreement.toFixed(2)
                                            color: root.riskColor(modelData.disagreement)
                                            font.pixelSize: 13
                                            font.bold: true
                                        }

                                        Label {
                                            Layout.preferredWidth: 178
                                            text: modelData.models
                                            color: root.primaryTextColor
                                            font.pixelSize: 12
                                            elide: Text.ElideRight
                                        }

                                        Label {
                                            Layout.fillWidth: true
                                            text: modelData.evidence
                                            color: root.mutedTextColor
                                            font.pixelSize: 12
                                            elide: Text.ElideRight
                                        }

                                        Rectangle {
                                            Layout.preferredWidth: 88
                                            implicitHeight: 28
                                            radius: 4
                                            color: "#24302d"
                                            border.color: root.borderColor

                                            Label {
                                                anchors.centerIn: parent
                                                text: modelData.state
                                                color: root.primaryTextColor
                                                font.pixelSize: 12
                                                font.bold: true
                                            }
                                        }
                                    }
                                }
                            }

                            Item {
                                Layout.fillHeight: true
                            }
                        }
                    }
                }
            }

            Rectangle {
                objectName: "rightDetailPanel"
                Layout.preferredWidth: 360
                Layout.fillHeight: true
                color: root.panelDeepColor
                border.color: root.borderColor
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 16

                    ColumnLayout {
                        objectName: "selectedEntityDetail"
                        Layout.fillWidth: true
                        spacing: 6

                        Label {
                            text: "Selected Entity"
                            color: root.mutedTextColor
                            font.pixelSize: 11
                            font.bold: true
                        }

                        Label {
                            text: "asset-alpha"
                            color: root.primaryTextColor
                            font.pixelSize: 22
                            font.bold: true
                        }

                        Label {
                            text: "Window 2026-01-01T00:05Z / synthetic fixture"
                            color: root.mutedTextColor
                            font.pixelSize: 12
                            wrapMode: Text.WordWrap
                        }
                    }

                    Rectangle {
                        objectName: "evidenceDetailPanel"
                        Layout.fillWidth: true
                        implicitHeight: 210
                        radius: 4
                        color: root.panelColor
                        border.color: root.borderColor

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 10

                            Label {
                                text: "Evidence"
                                color: root.primaryTextColor
                                font.pixelSize: 15
                                font.bold: true
                            }

                            Repeater {
                                model: [
                                    "Consensus risk supported by three detectors",
                                    "Dissent remains below triage threshold",
                                    "Evidence refs are coarse and synthetic",
                                    "No raw packet payload displayed"
                                ]

                                delegate: Label {
                                    Layout.fillWidth: true
                                    text: "- " + modelData
                                    color: root.mutedTextColor
                                    font.pixelSize: 12
                                    wrapMode: Text.WordWrap
                                }
                            }
                        }
                    }

                    Rectangle {
                        objectName: "analystActionPanel"
                        Layout.fillWidth: true
                        implicitHeight: 222
                        radius: 4
                        color: root.panelColor
                        border.color: root.borderColor

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 10

                            Label {
                                text: "Next Analyst Actions"
                                color: root.primaryTextColor
                                font.pixelSize: 15
                                font.bold: true
                            }

                            Repeater {
                                model: [
                                    "Open model evidence",
                                    "Inspect graph neighborhood",
                                    "Draft detection candidate",
                                    "Add investigation note"
                                ]

                                delegate: Button {
                                    id: actionButton
                                    Layout.fillWidth: true
                                    implicitHeight: 34
                                    text: modelData

                                    contentItem: Label {
                                        text: actionButton.text
                                        color: root.primaryTextColor
                                        font.pixelSize: 12
                                        verticalAlignment: Text.AlignVCenter
                                        elide: Text.ElideRight
                                    }

                                    background: Rectangle {
                                        radius: 4
                                        color: actionButton.hovered ? "#26312e" : "#202724"
                                        border.color: root.borderColor
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        objectName: "modelRegistrySnapshot"
                        Layout.fillWidth: true
                        implicitHeight: 108
                        radius: 4
                        color: root.panelColor
                        border.color: root.borderColor

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 6

                            Label {
                                text: "Registry Snapshot"
                                color: root.primaryTextColor
                                font.pixelSize: 15
                                font.bold: true
                            }

                            Label {
                                Layout.fillWidth: true
                                text: "4 detectors / 1 disagreement report / 0 exported artifacts"
                                color: root.mutedTextColor
                                font.pixelSize: 12
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    Item {
                        Layout.fillHeight: true
                    }
                }
            }
        }
    }
}
