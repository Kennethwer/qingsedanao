using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

namespace CyanBrainTrainer
{
    /// <summary>只监听 127.0.0.1 的行协议 TCP 服务，每个连接一个后台线程。</summary>
    internal sealed class RemoteServer
    {
        private readonly int port;
        private readonly string helloLine;
        private readonly Func<string, string> handler;
        private readonly List<TcpClient> clients = new List<TcpClient>();
        private TcpListener listener;
        private volatile bool running;

        public RemoteServer(int port, string helloLine, Func<string, string> handler)
        {
            this.port = port;
            this.helloLine = helloLine;
            this.handler = handler;
        }

        public int Port => port;

        public bool IsRunning => running;

        public string LastError { get; private set; }

        public int ClientCount
        {
            get
            {
                lock (clients)
                {
                    return clients.Count;
                }
            }
        }

        public bool Start()
        {
            try
            {
                listener = new TcpListener(IPAddress.Loopback, port);
                listener.Start();
            }
            catch (Exception e)
            {
                LastError = e.Message;
                Plugin.Log.LogError($"外部控制端口 {port} 监听失败：{e.Message}");
                return false;
            }

            running = true;
            new Thread(AcceptLoop) { IsBackground = true, Name = "CyanBrainTrainer-Accept" }.Start();
            Plugin.Log.LogInfo($"外部控制已开启：127.0.0.1:{port}");
            return true;
        }

        public void Stop()
        {
            running = false;
            try
            {
                listener?.Stop();
            }
            catch (Exception)
            {
                // 退出时忽略
            }
            lock (clients)
            {
                foreach (TcpClient client in clients)
                {
                    client.Close();
                }
                clients.Clear();
            }
        }

        private void AcceptLoop()
        {
            while (running)
            {
                TcpClient client;
                try
                {
                    client = listener.AcceptTcpClient();
                }
                catch (Exception e)
                {
                    if (!running)
                    {
                        return;
                    }
                    Plugin.Log.LogWarning($"接受连接失败：{e.Message}");
                    Thread.Sleep(200);
                    continue;
                }

                lock (clients)
                {
                    clients.Add(client);
                }
                new Thread(() => Serve(client)) { IsBackground = true, Name = "CyanBrainTrainer-Client" }.Start();
            }
        }

        private void Serve(TcpClient client)
        {
            UTF8Encoding utf8 = new UTF8Encoding(false);
            try
            {
                client.NoDelay = true;
                using (NetworkStream stream = client.GetStream())
                using (StreamReader reader = new StreamReader(stream, utf8))
                using (StreamWriter writer = new StreamWriter(stream, utf8) { NewLine = "\n", AutoFlush = true })
                {
                    writer.WriteLine(helloLine);
                    string line;
                    while (running && (line = reader.ReadLine()) != null)
                    {
                        string reply;
                        try
                        {
                            reply = handler(line);
                        }
                        catch (Exception e)
                        {
                            reply = Shared.Protocol.Err + " " + e.Message;
                        }
                        writer.WriteLine(reply);
                    }
                }
            }
            catch (IOException)
            {
                // 客户端断开
            }
            catch (ObjectDisposedException)
            {
                // 服务停止
            }
            catch (Exception e)
            {
                Plugin.Log.LogWarning($"外部连接异常：{e.Message}");
            }
            finally
            {
                lock (clients)
                {
                    clients.Remove(client);
                }
                client.Close();
            }
        }
    }
}
