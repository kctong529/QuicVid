#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QUdpSocket>
#include <QTimer>
#include <QLabel>
#include <QPushButton>
#include <QImage>
#include <QHostAddress>
#include <QWidget> // Added for base class of DrawingWidget

// Forward declaration of the custom drawing widget
class DrawingWidget;

// --- Configuration ---
const int UDP_PORT = 12345;
const int IMAGE_WIDTH = 320;
const int IMAGE_HEIGHT = 240;
const int STREAM_INTERVAL_MS = 50; // 20 FPS

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow();

private slots:
    // Server functions
    void toggleStreaming();
    void sendFrame();

    // Client function
    void processPendingDatagrams();

private:
    // Core setup methods
    void setupUI();
    void setupNetworking();

    // Networking
    QUdpSocket *serverSocket;
    QUdpSocket *clientSocket;
    QTimer *streamTimer;
    int frameCounter;

    DrawingWidget *drawingWidget;

    // UI elements (Server Panel)
    QPushButton *startButton;
    QLabel *serverStatusLabel;
    QLabel *serverSizeLabel;

    // UI elements (Client Panel)
    QLabel *clientImageLabel;
    QLabel *clientStatusLabel;
    QLabel *clientSizeLabel;
};

#endif // MAINWINDOW_H
