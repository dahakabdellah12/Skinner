using System;
using System.Buffers;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Xml.Linq;
using BrawlhallaSwz.Xml;

namespace BrawlhallaSwz;

public static partial class SwzUtils
{
    private const int COPY_STREAM_BUFFER_SIZE = 81920;

    internal static uint CalculateKeyChecksum(uint key, SwzRandom rand)
    {
        uint checksum = 0x2DF4A1CDu;
        uint rounds = key % 31 + 5;
        for (uint i = 0; i < rounds; ++i)
        {
            checksum ^= rand.Next();
        }
        return checksum;
    }

    // why is this not a standard function?
    internal static long CopyStream(Stream source, Stream destination, long bytes)
    {
        byte[] buffer = ArrayPool<byte>.Shared.Rent(COPY_STREAM_BUFFER_SIZE);
        try
        {
            long bytesRead = 0;
            while (bytesRead < bytes)
            {
                // read
                long wantToRead = bytes - bytesRead;
                int toRead = wantToRead > buffer.Length ? buffer.Length : (int)wantToRead;
                int read = source.Read(buffer.AsSpan(0, toRead));
                if (read == 0) break;
                //write
                destination.Write(buffer.AsSpan(0, read));

                bytesRead += read;
            }
            return bytesRead;
        }
        finally
        {
            ArrayPool<byte>.Shared.Return(buffer);
        }
    }

    internal static ValueTask<long> CopyStreamAsync(Stream source, Stream destination, long bytes, CancellationToken cancellationToken = default)
    {
        if (cancellationToken.IsCancellationRequested)
            return ValueTask.FromCanceled<long>(cancellationToken);

        return Core(source, destination, bytes, cancellationToken);

        static async ValueTask<long> Core(Stream source, Stream destination, long bytes, CancellationToken cancellationToken = default)
        {
            byte[] buffer = ArrayPool<byte>.Shared.Rent(COPY_STREAM_BUFFER_SIZE);
            try
            {
                long bytesRead = 0;
                while (bytesRead < bytes)
                {
                    // read
                    long wantToRead = bytes - bytesRead;
                    int toRead = wantToRead > buffer.Length ? buffer.Length : (int)wantToRead;
                    int read = await source.ReadAsync(buffer.AsMemory(0, toRead), cancellationToken).ConfigureAwait(false);
                    if (read == 0) break;
                    //write
                    await destination.WriteAsync(buffer.AsMemory(0, read), cancellationToken).ConfigureAwait(false);

                    bytesRead += read;
                }
                return bytesRead;
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(buffer);
            }
        }
    }

    public static bool KeyValidates(byte[] swzHeader8Bytes, uint key)
    {
        if (swzHeader8Bytes.Length < 8) return false;
        uint checksum = (uint)((swzHeader8Bytes[0] << 24) | (swzHeader8Bytes[1] << 16) | (swzHeader8Bytes[2] << 8) | swzHeader8Bytes[3]);
        uint seed = (uint)((swzHeader8Bytes[4] << 24) | (swzHeader8Bytes[5] << 16) | (swzHeader8Bytes[6] << 8) | swzHeader8Bytes[7]);
        var rand = new SwzRandom(key ^ seed);
        return CalculateKeyChecksum(key, rand) == checksum;
    }

    public static uint? FindSwzKey(string airSwfPath, string swzPath)
    {
        if (!File.Exists(airSwfPath) || !File.Exists(swzPath)) return null;
        byte[] swzHead = new byte[8];
        using (var f = new FileStream(swzPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        {
            if (f.Read(swzHead, 0, 8) < 8) return null;
        }

        // Test default known key first
        if (KeyValidates(swzHead, 154281100u)) return 154281100u;

        byte[] swfData;
        using (var afs = new FileStream(airSwfPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
        using (var ms = new MemoryStream())
        {
            afs.CopyTo(ms);
            swfData = ms.ToArray();
        }
        byte[] body;
        if (swfData.Length > 8 && swfData[0] == (byte)'C' && swfData[1] == (byte)'W' && swfData[2] == (byte)'S')
        {
            using var ms = new MemoryStream(swfData, 8, swfData.Length - 8);
            using var zlib = new System.IO.Compression.ZLibStream(ms, System.IO.Compression.CompressionMode.Decompress);
            using var outMs = new MemoryStream();
            zlib.CopyTo(outMs);
            body = outMs.ToArray();
        }
        else
        {
            body = swfData.Skip(8).ToArray();
        }

        for (int i = 0; i < body.Length - 5; i++)
        {
            uint val = 0;
            int shift = 0;
            for (int offset = 0; offset < 5; offset++)
            {
                byte b = body[i + offset];
                val |= (uint)(b & 0x7F) << shift;
                if ((b & 0x80) == 0) break;
                shift += 7;
                if (shift > 28) break;
            }
            if (val > 100000 && KeyValidates(swzHead, val))
            {
                return val;
            }
        }
        return null;
    }

    public static string GetFileName(string content)
    {
        if (content[0] == '<')
        {
            XElement element = BhXmlParser.ParseElement(content)!;
            return element.Name.LocalName switch
            {
                // use LevelName
                "LevelDesc" => "LevelDesc_" + element.Attribute("LevelName")!.Value,
                // use CutsceneName
                "CutsceneType" => "CutsceneType_" + element.Attribute("CutsceneName")!.Value,
                // use BattlePassID
                "QuestTypes" => "QuestTypes_" + element.Attribute("BattlePassID")!.Value,
                // use BattlePassID, but make sure to skip the template
                // this may need changing in the future...
                "BattlePassRewardTypes" => "BattlePassRewardTypes_" + element.Elements("BattlePassRewardType").Select((e) => e.Attribute("BattlePassID")!.Value).First((id) => id != "0"),
                _ => element.Name.LocalName,
            } + ".xml";
        }
        else
        {
            using StringReader stringReader = new(content);
            return stringReader.ReadLine()! + ".csv";
        }
    }
}