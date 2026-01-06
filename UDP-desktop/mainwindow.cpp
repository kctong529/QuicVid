#include "mainwindow.h"

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QBuffer>
#include <QImageWriter>
#include <QPainter>
#include <QDateTime>
#include <QPixmap>
#include <QDebug>
#include <QMouseEvent>
#include <QPen>

// --- New DrawingWidget Class ---
// This class handles the drawing canvas functionality using mouse events.
// It should ideally be in its own header/source file, but is included here
// for a self-contained implementation in mainwindow.cpp.
class DrawingWidget : public QWidget
{
public:
    DrawingWidget(QWidget *parent = nullptr) : QWidget(parent)
    {
        // Initialize the canvas image to match the stream size
        currentImage = QImage(IMAGE_WIDTH, IMAGE_HEIGHT, QImage::Format_RGB32);
        currentImage.fill(Qt::white); // Start with a white background
        setFixedSize(IMAGE_WIDTH, IMAGE_HEIGHT);
    }

    // Method to expose the current drawing for the server to stream
    QImage getImage() const {
        return currentImage;
    }

protected:
    void mousePressEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            lastPoint = event->pos();
            isDrawing = true;
        }
    }

    void mouseMoveEvent(QMouseEvent *event) override
    {
        if (isDrawing && event->buttons() & Qt::LeftButton) {
            QPainter painter(&currentImage);
            // Simple black pencil, thickness 3
            painter.setPen(QPen(Qt::black, 3, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.drawLine(lastPoint, event->pos());

            // Schedule repaint for the drawn area
            int rad = 3; // radius of the pen
            update(QRect(lastPoint, event->pos()).normalized().adjusted(-rad, -rad, rad, rad));

            lastPoint = event->pos();
        }
    }

    void mouseReleaseEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            isDrawing = false;
        }
    }

    void paintEvent(QPaintEvent *event) override
    {
        QPainter painter(this);
        QRect dirtyRect = event->rect();
        // Draw only the requested area of the underlying QImage
        painter.drawImage(dirtyRect, currentImage, dirtyRect);
    }

private:
    QImage currentImage;
    QPoint lastPoint;
    bool isDrawing = false;
};

// Assuming the following member exists in mainwindow.h:
// DrawingWidget *drawingWidget;

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent), frameCounter(0), drawingWidget(nullptr) // Initialize drawingWidget member
{
    setWindowTitle("Qt UDP Image Streamer (Drawing & Viewer)");
    setupUI();
    setupNetworking();
}

MainWindow::~MainWindow()
{
    // Sockets and Timer are parented to 'this', so they will be destroyed automatically.
}

void MainWindow::setupUI() {
    QWidget *centralWidget = new QWidget(this);
    setCentralWidget(centralWidget);

    QVBoxLayout *mainLayout = new QVBoxLayout(centralWidget);
    mainLayout->setSpacing(20);

    // --- Server (Sender) Panel ---
    QWidget *serverPanel = new QWidget();
    serverPanel->setObjectName("ServerPanel");
    serverPanel->setStyleSheet("#ServerPanel { background-color: #f0f8ff; border: 1px solid #cceeff; border-radius: 8px; padding: 10px; }");
    QVBoxLayout *serverLayout = new QVBoxLayout(serverPanel);

    QLabel *serverTitle = new QLabel("<h2>Drawing Server (Canvas)</h2>");
    serverTitle->setAlignment(Qt::AlignCenter);
    serverLayout->addWidget(serverTitle);

    // Instantiate the Drawing Widget for the Server Panel
    drawingWidget = new DrawingWidget(serverPanel);
    drawingWidget->setStyleSheet("border: 2px solid #4CAF50; border-radius: 5px; background-color: white;");
    serverLayout->addWidget(drawingWidget);

    startButton = new QPushButton("Start Stream (Server)");
    startButton->setStyleSheet("QPushButton { padding: 10px; font-size: 16px; background-color: #4CAF50; color: white; border-radius: 5px; } QPushButton:hover { background-color: #45a049; }");
    serverLayout->addWidget(startButton);

    serverStatusLabel = new QLabel("Status: Idle");
    serverSizeLabel = new QLabel("Last Size: 0 KB");
    serverLayout->addWidget(serverStatusLabel);
    serverLayout->addWidget(serverSizeLabel);

    mainLayout->addWidget(serverPanel);

    // --- Client (Receiver) Panel ---
    QWidget *clientPanel = new QWidget();
    clientPanel->setObjectName("ClientPanel");
    clientPanel->setStyleSheet("#ClientPanel { background-color: #fff0f5; border: 1px solid #ffcce6; border-radius: 8px; padding: 10px; }");
    QVBoxLayout *clientLayout = new QVBoxLayout(clientPanel);

    QLabel *clientTitle = new QLabel("<h2>Viewer Client (Receiver)</h2>");
    clientTitle->setAlignment(Qt::AlignCenter);
    clientLayout->addWidget(clientTitle);

    clientImageLabel = new QLabel("Awaiting UDP Image Stream...");
    clientImageLabel->setAlignment(Qt::AlignCenter);
    clientImageLabel->setFixedSize(IMAGE_WIDTH, IMAGE_HEIGHT); // Match expected image size
    clientImageLabel->setStyleSheet("background-color: #333; color: white; border: 2px solid #ccc; border-radius: 5px;");
    clientLayout->addWidget(clientImageLabel, 1); // Stretch factor 1

    clientStatusLabel = new QLabel("Status: Listening on port 12345");
    clientSizeLabel = new QLabel("Last Size: 0 KB");
    clientLayout->addWidget(clientStatusLabel);
    clientLayout->addWidget(clientSizeLabel);

    mainLayout->addWidget(clientPanel);

    // --- Connections ---
    connect(startButton, &QPushButton::clicked, this, &MainWindow::toggleStreaming);
}

void MainWindow::setupNetworking() {
    // --- Server Setup ---
    serverSocket = new QUdpSocket(this);
    streamTimer = new QTimer(this);

    // Timer triggers the sendFrame slot
    connect(streamTimer, &QTimer::timeout, this, &MainWindow::sendFrame);

    // --- Client Setup ---
    clientSocket = new QUdpSocket(this);

    // Bind the client socket to the target port
    // QHostAddress::LocalHost is typically the loopback address (127.0.0.1 or ::1)
    if (!clientSocket->bind(QHostAddress::LocalHost, UDP_PORT, QUdpSocket::ReuseAddressHint | QUdpSocket::ShareAddress)) {
        clientStatusLabel->setText(QString("Status: Failed to bind client socket: %1").arg(clientSocket->errorString()));
    } else {
        // Connect the readyRead signal to process incoming data
        connect(clientSocket, &QUdpSocket::readyRead, this, &MainWindow::processPendingDatagrams);
    }
}

void MainWindow::toggleStreaming() {
    if (streamTimer->isActive()) {
        streamTimer->stop();
        startButton->setText("Start Stream (Server)");
        serverStatusLabel->setText("Status: Stopped");
    } else {
        streamTimer->start(STREAM_INTERVAL_MS);
        startButton->setText("Stop Stream (Server)");

        // FIX: Explicitly create a QHostAddress object from the enum constant before calling .toString()
        serverStatusLabel->setText(QString("Status: Streaming to %1:%2").arg(QHostAddress(QHostAddress::LocalHost).toString()).arg(UDP_PORT));
    }
}

void MainWindow::sendFrame() {
    // 1. Get the current drawing image from the canvas
    if (!drawingWidget) {
        qWarning("Drawing widget not initialized.");
        return;
    }
    QImage image = drawingWidget->getImage();

    // Optionally draw metadata (like frame number) onto the image before sending
    QPainter painter(&image);
    painter.setPen(Qt::red);
    painter.setFont(QFont("Monospace", 10));
    painter.drawText(image.width() - 100, 20, QString("Frame: %1").arg(++frameCounter));
    painter.end();

    // 2. Compress image into a QByteArray (JPEG format)
    QByteArray datagram;
    QBuffer buffer(&datagram);
    buffer.open(QIODevice::WriteOnly);

    QImageWriter writer(&buffer, "jpg");
    // writer.setQuality(50); // Optional: Adjust quality for faster/smaller stream
    if (!writer.write(image)) {
        qWarning("Failed to write image to buffer.");
        return;
    }

    // 3. Send the datagram over UDP
    qint64 bytesSent = serverSocket->writeDatagram(datagram, QHostAddress::LocalHost, UDP_PORT);

    // Update server GUI
    serverSizeLabel->setText(QString("Last Size: %1 KB").arg(bytesSent / 1024.0, 0, 'f', 2));
}

void MainWindow::processPendingDatagrams() {
    while (clientSocket->hasPendingDatagrams()) {
        QByteArray datagram;
        datagram.resize(clientSocket->pendingDatagramSize());

        QHostAddress senderAddress;
        quint16 senderPort;

        clientSocket->readDatagram(datagram.data(), datagram.size(), &senderAddress, &senderPort);

        // 1. Convert QByteArray back into QPixmap
        QPixmap pixmap;
        if (pixmap.loadFromData(datagram, "JPG")) {
            // 2. Display the received pixmap
            clientImageLabel->setPixmap(pixmap.scaled(clientImageLabel->size(), Qt::KeepAspectRatio, Qt::SmoothTransformation));

            // Update client GUI
            clientStatusLabel->setText(QString("Status: Receiving from %1:%2").arg(senderAddress.toString()).arg(senderPort));
            clientSizeLabel->setText(QString("Last Size: %1 KB").arg(datagram.size() / 1024.0, 0, 'f', 2));

        } else {
            clientStatusLabel->setText("Status: Failed to load image data.");
        }
    }
}
