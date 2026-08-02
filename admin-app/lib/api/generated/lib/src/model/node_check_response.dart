//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'node_check_response.g.dart';

/// NodeCheckResponse
///
/// Properties:
/// * [architecture]
/// * [createdAt]
/// * [diskAvailableBytes]
/// * [failureCode]
/// * [failureMessage]
/// * [finishedAt]
/// * [id]
/// * [osName]
/// * [status]
@BuiltValue()
abstract class NodeCheckResponse implements Built<NodeCheckResponse, NodeCheckResponseBuilder> {
  @BuiltValueField(wireName: r'architecture')
  String? get architecture;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'disk_available_bytes')
  int? get diskAvailableBytes;

  @BuiltValueField(wireName: r'failure_code')
  String? get failureCode;

  @BuiltValueField(wireName: r'failure_message')
  String? get failureMessage;

  @BuiltValueField(wireName: r'finished_at')
  String? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'os_name')
  String? get osName;

  @BuiltValueField(wireName: r'status')
  String get status;

  NodeCheckResponse._();

  factory NodeCheckResponse([void updates(NodeCheckResponseBuilder b)]) = _$NodeCheckResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(NodeCheckResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<NodeCheckResponse> get serializer => _$NodeCheckResponseSerializer();
}

class _$NodeCheckResponseSerializer implements PrimitiveSerializer<NodeCheckResponse> {
  @override
  final Iterable<Type> types = const [NodeCheckResponse, _$NodeCheckResponse];

  @override
  final String wireName = r'NodeCheckResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    NodeCheckResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.architecture != null) {
      yield r'architecture';
      yield serializers.serialize(
        object.architecture,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.diskAvailableBytes != null) {
      yield r'disk_available_bytes';
      yield serializers.serialize(
        object.diskAvailableBytes,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.failureCode != null) {
      yield r'failure_code';
      yield serializers.serialize(
        object.failureCode,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.failureMessage != null) {
      yield r'failure_message';
      yield serializers.serialize(
        object.failureMessage,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.finishedAt != null) {
      yield r'finished_at';
      yield serializers.serialize(
        object.finishedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.osName != null) {
      yield r'os_name';
      yield serializers.serialize(
        object.osName,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    NodeCheckResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required NodeCheckResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'architecture':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.architecture = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'disk_available_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.diskAvailableBytes = valueDes;
          break;
        case r'failure_code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.failureCode = valueDes;
          break;
        case r'failure_message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.failureMessage = valueDes;
          break;
        case r'finished_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.finishedAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'os_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.osName = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  NodeCheckResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = NodeCheckResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
