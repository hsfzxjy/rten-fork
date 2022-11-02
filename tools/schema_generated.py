# automatically generated by the FlatBuffers compiler, do not modify

# namespace: 

import flatbuffers
from flatbuffers.compat import import_numpy
np = import_numpy()

class OperatorType(object):
    Add = 0
    BatchNormalization = 1
    Clip = 2
    Concat = 3
    Conv2d = 4
    ConvTranspose2d = 5
    Gather = 6
    Gemm = 7
    GlobalAveragePool = 8
    LeakyRelu = 9
    MatMul = 10
    MaxPool2d = 11
    Mul = 12
    Pad2d = 13
    Relu = 14
    Reshape = 15
    Shape = 16
    Sigmoid = 17
    Slice = 18
    Unsqueeze = 19


class PadMode(object):
    Same = 0
    Fixed = 1


class OperatorAttrs(object):
    NONE = 0
    BatchNormalizationAttrs = 1
    ClipAttrs = 2
    ConcatAttrs = 3
    Conv2dAttrs = 4
    ConvTranspose2dAttrs = 5
    GatherAttrs = 6
    GemmAttrs = 7
    LeakyReluAttrs = 8
    MaxPool2dAttrs = 9
    Pad2dAttrs = 10
    UnsqueezeAttrs = 11


class NodeKind(object):
    NONE = 0
    OperatorNode = 1
    ConstantNode = 2
    ValueNode = 3


class ConstantData(object):
    NONE = 0
    FloatData = 1
    IntData = 2


class BatchNormalizationAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = BatchNormalizationAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsBatchNormalizationAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def BatchNormalizationAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # BatchNormalizationAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # BatchNormalizationAttrs
    def Epsilon(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Float32Flags, o + self._tab.Pos)
        return 0.0

def BatchNormalizationAttrsStart(builder): builder.StartObject(1)
def BatchNormalizationAttrsAddEpsilon(builder, epsilon): builder.PrependFloat32Slot(0, epsilon, 0.0)
def BatchNormalizationAttrsEnd(builder): return builder.EndObject()


class ClipAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = ClipAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsClipAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def ClipAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # ClipAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # ClipAttrs
    def Min(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Float32Flags, o + self._tab.Pos)
        return 0.0

    # ClipAttrs
    def Max(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Float32Flags, o + self._tab.Pos)
        return 0.0

def ClipAttrsStart(builder): builder.StartObject(2)
def ClipAttrsAddMin(builder, min): builder.PrependFloat32Slot(0, min, 0.0)
def ClipAttrsAddMax(builder, max): builder.PrependFloat32Slot(1, max, 0.0)
def ClipAttrsEnd(builder): return builder.EndObject()


class ConcatAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = ConcatAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsConcatAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def ConcatAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # ConcatAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # ConcatAttrs
    def Dim(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

def ConcatAttrsStart(builder): builder.StartObject(1)
def ConcatAttrsAddDim(builder, dim): builder.PrependUint32Slot(0, dim, 0)
def ConcatAttrsEnd(builder): return builder.EndObject()


class Conv2dAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = Conv2dAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsConv2dAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def Conv2dAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # Conv2dAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # Conv2dAttrs
    def PadMode(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Int8Flags, o + self._tab.Pos)
        return 0

    # Conv2dAttrs
    def PadHorizontal(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # Conv2dAttrs
    def PadVertical(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(8))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # Conv2dAttrs
    def Groups(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # Conv2dAttrs
    def Stride(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(12))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

def Conv2dAttrsStart(builder): builder.StartObject(5)
def Conv2dAttrsAddPadMode(builder, padMode): builder.PrependInt8Slot(0, padMode, 0)
def Conv2dAttrsAddPadHorizontal(builder, padHorizontal): builder.PrependUint32Slot(1, padHorizontal, 0)
def Conv2dAttrsAddPadVertical(builder, padVertical): builder.PrependUint32Slot(2, padVertical, 0)
def Conv2dAttrsAddGroups(builder, groups): builder.PrependUint32Slot(3, groups, 0)
def Conv2dAttrsAddStride(builder, stride): builder.PrependUint32Slot(4, stride, 0)
def Conv2dAttrsEnd(builder): return builder.EndObject()


class ConvTranspose2dAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = ConvTranspose2dAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsConvTranspose2dAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def ConvTranspose2dAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # ConvTranspose2dAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # ConvTranspose2dAttrs
    def Stride(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

def ConvTranspose2dAttrsStart(builder): builder.StartObject(1)
def ConvTranspose2dAttrsAddStride(builder, stride): builder.PrependUint32Slot(0, stride, 0)
def ConvTranspose2dAttrsEnd(builder): return builder.EndObject()


class GatherAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = GatherAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsGatherAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def GatherAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # GatherAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # GatherAttrs
    def Axis(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

def GatherAttrsStart(builder): builder.StartObject(1)
def GatherAttrsAddAxis(builder, axis): builder.PrependUint32Slot(0, axis, 0)
def GatherAttrsEnd(builder): return builder.EndObject()


class GemmAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = GemmAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsGemmAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def GemmAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # GemmAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # GemmAttrs
    def Alpha(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Float32Flags, o + self._tab.Pos)
        return 0.0

    # GemmAttrs
    def Beta(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Float32Flags, o + self._tab.Pos)
        return 0.0

    # GemmAttrs
    def TransposeA(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(8))
        if o != 0:
            return bool(self._tab.Get(flatbuffers.number_types.BoolFlags, o + self._tab.Pos))
        return False

    # GemmAttrs
    def TransposeB(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        if o != 0:
            return bool(self._tab.Get(flatbuffers.number_types.BoolFlags, o + self._tab.Pos))
        return False

def GemmAttrsStart(builder): builder.StartObject(4)
def GemmAttrsAddAlpha(builder, alpha): builder.PrependFloat32Slot(0, alpha, 0.0)
def GemmAttrsAddBeta(builder, beta): builder.PrependFloat32Slot(1, beta, 0.0)
def GemmAttrsAddTransposeA(builder, transposeA): builder.PrependBoolSlot(2, transposeA, 0)
def GemmAttrsAddTransposeB(builder, transposeB): builder.PrependBoolSlot(3, transposeB, 0)
def GemmAttrsEnd(builder): return builder.EndObject()


class LeakyReluAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = LeakyReluAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsLeakyReluAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def LeakyReluAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # LeakyReluAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # LeakyReluAttrs
    def Alpha(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Float32Flags, o + self._tab.Pos)
        return 0.0

def LeakyReluAttrsStart(builder): builder.StartObject(1)
def LeakyReluAttrsAddAlpha(builder, alpha): builder.PrependFloat32Slot(0, alpha, 0.0)
def LeakyReluAttrsEnd(builder): return builder.EndObject()


class MaxPool2dAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = MaxPool2dAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsMaxPool2dAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def MaxPool2dAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # MaxPool2dAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # MaxPool2dAttrs
    def KernelSize(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # MaxPool2dAttrs
    def PadMode(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Int8Flags, o + self._tab.Pos)
        return 0

    # MaxPool2dAttrs
    def PadHorizontal(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(8))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # MaxPool2dAttrs
    def PadVertical(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # MaxPool2dAttrs
    def Stride(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(12))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

def MaxPool2dAttrsStart(builder): builder.StartObject(5)
def MaxPool2dAttrsAddKernelSize(builder, kernelSize): builder.PrependUint32Slot(0, kernelSize, 0)
def MaxPool2dAttrsAddPadMode(builder, padMode): builder.PrependInt8Slot(1, padMode, 0)
def MaxPool2dAttrsAddPadHorizontal(builder, padHorizontal): builder.PrependUint32Slot(2, padHorizontal, 0)
def MaxPool2dAttrsAddPadVertical(builder, padVertical): builder.PrependUint32Slot(3, padVertical, 0)
def MaxPool2dAttrsAddStride(builder, stride): builder.PrependUint32Slot(4, stride, 0)
def MaxPool2dAttrsEnd(builder): return builder.EndObject()


class Pad2dAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = Pad2dAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsPad2dAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def Pad2dAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # Pad2dAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # Pad2dAttrs
    def PadLeft(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # Pad2dAttrs
    def PadRight(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # Pad2dAttrs
    def PadTop(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(8))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

    # Pad2dAttrs
    def PadBottom(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, o + self._tab.Pos)
        return 0

def Pad2dAttrsStart(builder): builder.StartObject(4)
def Pad2dAttrsAddPadLeft(builder, padLeft): builder.PrependUint32Slot(0, padLeft, 0)
def Pad2dAttrsAddPadRight(builder, padRight): builder.PrependUint32Slot(1, padRight, 0)
def Pad2dAttrsAddPadTop(builder, padTop): builder.PrependUint32Slot(2, padTop, 0)
def Pad2dAttrsAddPadBottom(builder, padBottom): builder.PrependUint32Slot(3, padBottom, 0)
def Pad2dAttrsEnd(builder): return builder.EndObject()


class UnsqueezeAttrs(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = UnsqueezeAttrs()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsUnsqueezeAttrs(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def UnsqueezeAttrsBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # UnsqueezeAttrs
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # UnsqueezeAttrs
    def Axes(self, j):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            a = self._tab.Vector(o)
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, a + flatbuffers.number_types.UOffsetTFlags.py_type(j * 4))
        return 0

    # UnsqueezeAttrs
    def AxesAsNumpy(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.GetVectorAsNumpy(flatbuffers.number_types.Uint32Flags, o)
        return 0

    # UnsqueezeAttrs
    def AxesLength(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.VectorLen(o)
        return 0

    # UnsqueezeAttrs
    def AxesIsNone(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        return o == 0

def UnsqueezeAttrsStart(builder): builder.StartObject(1)
def UnsqueezeAttrsAddAxes(builder, axes): builder.PrependUOffsetTRelativeSlot(0, flatbuffers.number_types.UOffsetTFlags.py_type(axes), 0)
def UnsqueezeAttrsStartAxesVector(builder, numElems): return builder.StartVector(4, numElems, 4)
def UnsqueezeAttrsEnd(builder): return builder.EndObject()


class OperatorNode(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = OperatorNode()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsOperatorNode(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def OperatorNodeBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # OperatorNode
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # OperatorNode
    def Type(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Int8Flags, o + self._tab.Pos)
        return 0

    # OperatorNode
    def AttrsType(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint8Flags, o + self._tab.Pos)
        return 0

    # OperatorNode
    def Attrs(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(8))
        if o != 0:
            from flatbuffers.table import Table
            obj = Table(bytearray(), 0)
            self._tab.Union(obj, o)
            return obj
        return None

    # OperatorNode
    def Inputs(self, j):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        if o != 0:
            a = self._tab.Vector(o)
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, a + flatbuffers.number_types.UOffsetTFlags.py_type(j * 4))
        return 0

    # OperatorNode
    def InputsAsNumpy(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        if o != 0:
            return self._tab.GetVectorAsNumpy(flatbuffers.number_types.Uint32Flags, o)
        return 0

    # OperatorNode
    def InputsLength(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        if o != 0:
            return self._tab.VectorLen(o)
        return 0

    # OperatorNode
    def InputsIsNone(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(10))
        return o == 0

def OperatorNodeStart(builder): builder.StartObject(4)
def OperatorNodeAddType(builder, type): builder.PrependInt8Slot(0, type, 0)
def OperatorNodeAddAttrsType(builder, attrsType): builder.PrependUint8Slot(1, attrsType, 0)
def OperatorNodeAddAttrs(builder, attrs): builder.PrependUOffsetTRelativeSlot(2, flatbuffers.number_types.UOffsetTFlags.py_type(attrs), 0)
def OperatorNodeAddInputs(builder, inputs): builder.PrependUOffsetTRelativeSlot(3, flatbuffers.number_types.UOffsetTFlags.py_type(inputs), 0)
def OperatorNodeStartInputsVector(builder, numElems): return builder.StartVector(4, numElems, 4)
def OperatorNodeEnd(builder): return builder.EndObject()


class FloatData(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = FloatData()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsFloatData(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def FloatDataBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # FloatData
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # FloatData
    def Data(self, j):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            a = self._tab.Vector(o)
            return self._tab.Get(flatbuffers.number_types.Float32Flags, a + flatbuffers.number_types.UOffsetTFlags.py_type(j * 4))
        return 0

    # FloatData
    def DataAsNumpy(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.GetVectorAsNumpy(flatbuffers.number_types.Float32Flags, o)
        return 0

    # FloatData
    def DataLength(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.VectorLen(o)
        return 0

    # FloatData
    def DataIsNone(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        return o == 0

def FloatDataStart(builder): builder.StartObject(1)
def FloatDataAddData(builder, data): builder.PrependUOffsetTRelativeSlot(0, flatbuffers.number_types.UOffsetTFlags.py_type(data), 0)
def FloatDataStartDataVector(builder, numElems): return builder.StartVector(4, numElems, 4)
def FloatDataEnd(builder): return builder.EndObject()


class IntData(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = IntData()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsIntData(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def IntDataBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # IntData
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # IntData
    def Data(self, j):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            a = self._tab.Vector(o)
            return self._tab.Get(flatbuffers.number_types.Int32Flags, a + flatbuffers.number_types.UOffsetTFlags.py_type(j * 4))
        return 0

    # IntData
    def DataAsNumpy(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.GetVectorAsNumpy(flatbuffers.number_types.Int32Flags, o)
        return 0

    # IntData
    def DataLength(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.VectorLen(o)
        return 0

    # IntData
    def DataIsNone(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        return o == 0

def IntDataStart(builder): builder.StartObject(1)
def IntDataAddData(builder, data): builder.PrependUOffsetTRelativeSlot(0, flatbuffers.number_types.UOffsetTFlags.py_type(data), 0)
def IntDataStartDataVector(builder, numElems): return builder.StartVector(4, numElems, 4)
def IntDataEnd(builder): return builder.EndObject()


class ConstantNode(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = ConstantNode()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsConstantNode(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def ConstantNodeBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # ConstantNode
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # ConstantNode
    def Shape(self, j):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            a = self._tab.Vector(o)
            return self._tab.Get(flatbuffers.number_types.Uint32Flags, a + flatbuffers.number_types.UOffsetTFlags.py_type(j * 4))
        return 0

    # ConstantNode
    def ShapeAsNumpy(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.GetVectorAsNumpy(flatbuffers.number_types.Uint32Flags, o)
        return 0

    # ConstantNode
    def ShapeLength(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.VectorLen(o)
        return 0

    # ConstantNode
    def ShapeIsNone(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        return o == 0

    # ConstantNode
    def DataType(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint8Flags, o + self._tab.Pos)
        return 0

    # ConstantNode
    def Data(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(8))
        if o != 0:
            from flatbuffers.table import Table
            obj = Table(bytearray(), 0)
            self._tab.Union(obj, o)
            return obj
        return None

def ConstantNodeStart(builder): builder.StartObject(3)
def ConstantNodeAddShape(builder, shape): builder.PrependUOffsetTRelativeSlot(0, flatbuffers.number_types.UOffsetTFlags.py_type(shape), 0)
def ConstantNodeStartShapeVector(builder, numElems): return builder.StartVector(4, numElems, 4)
def ConstantNodeAddDataType(builder, dataType): builder.PrependUint8Slot(1, dataType, 0)
def ConstantNodeAddData(builder, data): builder.PrependUOffsetTRelativeSlot(2, flatbuffers.number_types.UOffsetTFlags.py_type(data), 0)
def ConstantNodeEnd(builder): return builder.EndObject()


class ValueNode(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = ValueNode()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsValueNode(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def ValueNodeBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # ValueNode
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

def ValueNodeStart(builder): builder.StartObject(0)
def ValueNodeEnd(builder): return builder.EndObject()


class Node(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = Node()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsNode(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def NodeBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # Node
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # Node
    def Id(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.String(o + self._tab.Pos)
        return None

    # Node
    def DataType(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Uint8Flags, o + self._tab.Pos)
        return 0

    # Node
    def Data(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(8))
        if o != 0:
            from flatbuffers.table import Table
            obj = Table(bytearray(), 0)
            self._tab.Union(obj, o)
            return obj
        return None

def NodeStart(builder): builder.StartObject(3)
def NodeAddId(builder, id): builder.PrependUOffsetTRelativeSlot(0, flatbuffers.number_types.UOffsetTFlags.py_type(id), 0)
def NodeAddDataType(builder, dataType): builder.PrependUint8Slot(1, dataType, 0)
def NodeAddData(builder, data): builder.PrependUOffsetTRelativeSlot(2, flatbuffers.number_types.UOffsetTFlags.py_type(data), 0)
def NodeEnd(builder): return builder.EndObject()


class Graph(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = Graph()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsGraph(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def GraphBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # Graph
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # Graph
    def Nodes(self, j):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            x = self._tab.Vector(o)
            x += flatbuffers.number_types.UOffsetTFlags.py_type(j) * 4
            x = self._tab.Indirect(x)
            obj = Node()
            obj.Init(self._tab.Bytes, x)
            return obj
        return None

    # Graph
    def NodesLength(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.VectorLen(o)
        return 0

    # Graph
    def NodesIsNone(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        return o == 0

def GraphStart(builder): builder.StartObject(1)
def GraphAddNodes(builder, nodes): builder.PrependUOffsetTRelativeSlot(0, flatbuffers.number_types.UOffsetTFlags.py_type(nodes), 0)
def GraphStartNodesVector(builder, numElems): return builder.StartVector(4, numElems, 4)
def GraphEnd(builder): return builder.EndObject()


class Model(object):
    __slots__ = ['_tab']

    @classmethod
    def GetRootAs(cls, buf, offset=0):
        n = flatbuffers.encode.Get(flatbuffers.packer.uoffset, buf, offset)
        x = Model()
        x.Init(buf, n + offset)
        return x

    @classmethod
    def GetRootAsModel(cls, buf, offset=0):
        """This method is deprecated. Please switch to GetRootAs."""
        return cls.GetRootAs(buf, offset)
    @classmethod
    def ModelBufferHasIdentifier(cls, buf, offset, size_prefixed=False):
        return flatbuffers.util.BufferHasIdentifier(buf, offset, b"\x4D\x4F\x44\x4C", size_prefixed=size_prefixed)

    # Model
    def Init(self, buf, pos):
        self._tab = flatbuffers.table.Table(buf, pos)

    # Model
    def SchemaVersion(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(4))
        if o != 0:
            return self._tab.Get(flatbuffers.number_types.Int32Flags, o + self._tab.Pos)
        return 0

    # Model
    def Graph(self):
        o = flatbuffers.number_types.UOffsetTFlags.py_type(self._tab.Offset(6))
        if o != 0:
            x = self._tab.Indirect(o + self._tab.Pos)
            obj = Graph()
            obj.Init(self._tab.Bytes, x)
            return obj
        return None

def ModelStart(builder): builder.StartObject(2)
def ModelAddSchemaVersion(builder, schemaVersion): builder.PrependInt32Slot(0, schemaVersion, 0)
def ModelAddGraph(builder, graph): builder.PrependUOffsetTRelativeSlot(1, flatbuffers.number_types.UOffsetTFlags.py_type(graph), 0)
def ModelEnd(builder): return builder.EndObject()


